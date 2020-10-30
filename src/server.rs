use crate::allocator::ConsulIpAllocator;
use crate::cni::{CniRequest, IpamResponse};
use anyhow::Result;
use clokwerk::Scheduler;
use serde_json;
use std::collections::BTreeMap as Map;
use std::io::Write;
use std::io::{BufRead, BufReader};
use std::os::unix::net::{UnixListener, UnixStream};
use std::thread;

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

    pub fn run(self) -> Result<()> {
        let listener = UnixListener::bind("/tmp/cni-ipam-consul.sock").unwrap();

        ctrlc::set_handler(move || {
            println!("Interrupted, shutting down");
            std::fs::remove_file("/tmp/cni-ipam-consul.sock");

            std::process::exit(0);
        })
        .expect("Error setting Ctrl-C handler");

        for mut stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    thread::spawn(|| handle_client(stream));
                }
                Err(err) => {
                    println!("Error: {}", err);
                    break;
                }
            }
        }

        Ok(())
    }
}

fn handle_client(mut stream: UnixStream) {
    if let Some(e) = handle_stream(stream).err() {
        println!("Failed to handle request: {}", e);
    }
}

fn handle_stream(mut stream: UnixStream) -> Result<()> {
    let mut writer = stream.try_clone().expect("Could not copy stream");
    let reader = BufReader::new(stream);

    for line in reader.lines() {
        let req: CniRequest = serde_json::from_str(line?.as_str())?;
        println!("Got CNI request {:?}", req);

        let resp = exec_request(req)?;

        println!("Sending IPAM response: {:?}", resp);

        writer.write_all(serde_json::to_string(&resp)?.as_bytes())?;
        writer.write_all("\n".as_bytes())?;
    }

    Ok(())
}

fn exec_request(req: CniRequest) -> Result<IpamResponse> {
    Ok(IpamResponse::new(Vec::new(), Vec::new()))
}
