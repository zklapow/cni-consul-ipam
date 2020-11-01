use crate::allocator::ConsulIpAllocator;
use crate::cni::{CniRequest, IpResponse, IpamResponse};
use anyhow::Result;
use cidr::{Cidr, Ipv4Cidr};
use clokwerk::Scheduler;
use log::LevelFilter;
use log4rs::append::file::FileAppender;
use log4rs::config::{Appender, Config, Logger, Root};
use log4rs::encode::pattern::PatternEncoder;
use log4rs::filter::threshold::ThresholdFilter;
use serde_json;
use std::borrow::Borrow;
use std::collections::BTreeMap as Map;
use std::io::Write;
use std::io::{BufRead, BufReader};
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

pub(crate) struct ConsulIpamServer {
    allocator: ConsulIpAllocator,
    scheduler: Scheduler,
}

impl ConsulIpamServer {
    pub fn new() -> Result<ConsulIpamServer> {
        Ok(ConsulIpamServer {
            allocator: ConsulIpAllocator::new()?,
            scheduler: Scheduler::new(),
        })
    }

    pub fn run(mut self) -> Result<()> {
        init_logging();

        let listener = UnixListener::bind("/tmp/cni-ipam-consul.sock").unwrap();

        self.allocator.start(&mut self.scheduler);

        let thread_handle = self.scheduler.watch_thread(Duration::from_millis(100));

        let stop_allocator = self.allocator.clone();
        ctrlc::set_handler(move || {
            println!("Interrupted, shutting down");
            std::fs::remove_file("/tmp/cni-ipam-consul.sock");
            stop_allocator.stop();

            std::process::exit(0);
        })
        .expect("Error setting Ctrl-C handler");

        for mut stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let client_allocator = self.allocator.clone();
                    thread::spawn(|| handle_client(stream, client_allocator));
                }
                Err(err) => {
                    println!("Error: {}", err);
                    break;
                }
            }
        }

        thread_handle.stop();

        Ok(())
    }
}

fn handle_client(mut stream: UnixStream, allocator: ConsulIpAllocator) {
    if let Some(e) = handle_stream(stream, allocator).err() {
        println!("Failed to handle request: {}", e);
    }
}

fn handle_stream(mut stream: UnixStream, allocator: ConsulIpAllocator) -> Result<()> {
    let mut writer = stream.try_clone().expect("Could not copy stream");

    let mut reader = BufReader::new(stream);

    let mut buf: String = String::new();
    let _ = reader.read_line(&mut buf)?;

    let req: CniRequest = serde_json::from_str(buf.as_str())?;
    println!("Got CNI request {:?}", req);

    let resp = exec_request(req, allocator)?;

    println!("Sending IPAM response: {:?}", resp);

    writer.write_all(serde_json::to_string(&resp)?.as_bytes())?;
    writer.write_all("\n".as_bytes())?;

    Ok(())
}

fn exec_request(req: CniRequest, allocator: ConsulIpAllocator) -> Result<IpamResponse> {
    let allocated_addr =
        allocator.allocate_from(req.config.name, req.container_id, req.config.ipam.subnet)?;

    let addr_cidr = Ipv4Cidr::new_host(allocated_addr);

    let ip_resp = IpResponse {
        version: String::from("4"),
        address: addr_cidr,
        gateway: None,
        interface: None,
    };

    Ok(IpamResponse::new(
        vec![ip_resp],
        req.config.ipam.routes,
        req.config.dns,
    ))
}

fn init_logging() {
    let logfile = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new("{d} - {l} - {L} - {m}{n}")))
        .build("/tmp/log/consul-ipam-server.log")
        .unwrap();

    let config = Config::builder()
        .appender(
            Appender::builder()
                .filter(Box::new(ThresholdFilter::new(LevelFilter::Info)))
                .build("logfile", Box::new(logfile)),
        )
        .build(
            Root::builder()
                .appender("logfile")
                .build(LevelFilter::Trace),
        )
        .unwrap();
    let _handle = log4rs::init_config(config).unwrap();
}
