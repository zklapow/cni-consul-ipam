use crate::{CniRequest, IpamResponse};
use anyhow::Result;
use serde_json;
use std::io::Write;
use std::io::{BufRead, BufReader};
use std::os::unix::net::{UnixListener, UnixStream};
use std::thread;

pub fn run_server() -> Result<()> {
    let listener = UnixListener::bind("/tmp/cni-ipam-consul.sock").unwrap();

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
    Ok(IpamResponse {
        ip: "127.0.0.1".to_string(),
    })
}
