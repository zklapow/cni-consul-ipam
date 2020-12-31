use crate::allocator::ConsulIpAllocator;
use crate::cni::{CniRequest, IpResponse, IpamResponse};
use anyhow::{anyhow, Result};
use cidr::{Cidr, Ipv4Cidr};
use clokwerk::Scheduler;
use listenfd::ListenFd;
use log::{error, info, warn, LevelFilter};
use log4rs::append::console::ConsoleAppender;
use log4rs::append::file::FileAppender;
use log4rs::config::{Appender, Config, Logger, Root};
use log4rs::encode::pattern::PatternEncoder;
use log4rs::filter::threshold::ThresholdFilter;
use serde_json;
use std::borrow::Borrow;
use std::collections::BTreeMap as Map;
use std::io::Write;
use std::io::{BufRead, BufReader};
use std::ops::DerefMut;
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

pub(crate) struct ConsulIpamServer {
    allocator: Arc<Mutex<ConsulIpAllocator>>,
    scheduler: Scheduler,
}

impl ConsulIpamServer {
    pub fn new() -> Result<ConsulIpamServer> {
        let allocator = ConsulIpAllocator::new()?;
        let mut scheduler = Scheduler::new();

        allocator.start(&mut scheduler);
        Ok(ConsulIpamServer {
            allocator: Arc::new(Mutex::new(allocator)),
            scheduler,
        })
    }

    pub fn run(mut self) -> Result<()> {
        init_logging();

        let mut listenfd = ListenFd::from_env();
        let listener = listenfd
            .take_unix_listener(0)?
            .unwrap_or_else(|| UnixListener::bind("/tmp/cni-ipam-consul.sock").unwrap());

        let thread_handle = self.scheduler.watch_thread(Duration::from_millis(100));

        let stop_allocator = self.allocator.clone();
        ctrlc::set_handler(move || {
            warn!("Interrupted, shutting down");
            std::fs::remove_file("/tmp/cni-ipam-consul.sock");

            let alloc = stop_allocator.lock().unwrap();
            alloc.stop();

            std::process::exit(0);
        })
        .expect("Error setting Ctrl-C handler");

        for mut stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let alloc_lock = self.allocator.clone();
                    thread::spawn(|| handle_client(stream, alloc_lock));
                }
                Err(err) => {
                    error!("Error: {}", err);
                    break;
                }
            }
        }

        thread_handle.stop();

        Ok(())
    }
}

fn handle_client(mut stream: UnixStream, allocator: Arc<Mutex<ConsulIpAllocator>>) {
    let mut alloc = allocator.lock().unwrap();
    if let Some(e) = handle_stream(stream, alloc.deref_mut()).err() {
        error!("Failed to handle request: {}", e);
    }
}

fn handle_stream(mut stream: UnixStream, allocator: &mut ConsulIpAllocator) -> Result<()> {
    let mut writer = stream.try_clone().expect("Could not copy stream");

    let mut reader = BufReader::new(stream);

    let mut buf: String = String::new();
    let _ = reader.read_line(&mut buf)?;

    let req: CniRequest = serde_json::from_str(buf.as_str())?;
    info!("Got CNI request {:?}", req);

    if let Some(resp) = exec_request(req, allocator)? {
        info!("Sending IPAM response: {:?}", resp);

        writer.write_all(serde_json::to_string(&resp)?.as_bytes())?;
        writer.write_all("\n".as_bytes())?;
    } else {
        writer.write_all("\n".as_bytes())?;
    }

    Ok(())
}

fn exec_request(
    req: CniRequest,
    allocator: &mut ConsulIpAllocator,
) -> Result<Option<IpamResponse>> {
    match req.command.to_lowercase().as_str() {
        "add" => exec_add(req, allocator).map(|v| Some(v)),
        "del" => exec_del(req, allocator).map(|_| None),
        _ => Err(anyhow!("Unknown command")),
    }
}

fn exec_del(req: CniRequest, allocator: &mut ConsulIpAllocator) -> Result<()> {
    allocator.release_from(req.config.name, req.container_id);
    Ok(())
}

fn exec_add(req: CniRequest, allocator: &mut ConsulIpAllocator) -> Result<IpamResponse> {
    let net_path = req.config.path.unwrap_or(req.config.name);
    let allocated_addr =
        allocator.allocate_from(net_path, req.container_id, req.config.ipam.subnet)?;

    let addr_cidr = Ipv4Cidr::new_host(allocated_addr);

    let ip_resp = IpResponse {
        version: String::from("4"),
        address: addr_cidr,
        gateway: Some(req.config.ipam.gateway),
        interface: None,
    };

    Ok(IpamResponse::new(
        vec![ip_resp],
        req.config.ipam.routes,
        req.config.dns,
    ))
}

fn init_logging() {
    let stdout = ConsoleAppender::builder()
        .encoder(Box::new(PatternEncoder::new(
            "{d} - {l} - {f}:{L} - {m}{n}",
        )))
        .build();

    let config = Config::builder()
        .appender(
            Appender::builder()
                .filter(Box::new(ThresholdFilter::new(LevelFilter::Info)))
                .build("stdout", Box::new(stdout)),
        )
        .build(Root::builder().appender("stdout").build(LevelFilter::Trace))
        .unwrap();
    let _handle = log4rs::init_config(config).unwrap();
}
