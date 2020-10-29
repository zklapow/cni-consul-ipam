use crate::CniRequest;
use anyhow::Result;
use serde_json;
use std::io::{BufRead, BufReader};
use std::os::unix::net::{UnixListener, UnixStream};
use std::thread;

pub fn run_server() -> Result<()> {
    let listener = UnixListener::bind("/tmp/cni-ipam-consul.sock").unwrap();

    for stream in listener.incoming() {
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

fn handle_client(stream: UnixStream) {
    if let Some(e) = handle_req(stream).err() {
        println!("Failed to handle request: {}", e);
    }
}

fn handle_req(stream: UnixStream) -> Result<()> {
    let stream = BufReader::new(stream);

    for line in stream.lines() {
        let req: CniRequest = serde_json::from_str(line?.as_str())?;
        println!("Got CNI request {:?}", req);
    }

    Ok(())
}
