use crate::CniRequest;
use anyhow::{anyhow, Error, Result};
use serde_json;
use std::io::Write;
use std::os::unix::net::UnixStream;

pub fn run_client() -> Result<()> {
    let req = get_request_from_env();

    match req.command.to_lowercase().as_str() {
        "add" => client_add(req),
        "del" => client_del(req),
        _ => Err(anyhow!("Unknown command {}", req.command)),
    }
}

pub fn client_add(req: CniRequest) -> Result<()> {
    println!("Container ID: {}", req.container_id);
    send_request(req)?;
    Ok(())
}

pub fn client_del(req: CniRequest) -> Result<()> {
    Ok(())
}

pub fn send_request(req: CniRequest) -> Result<()> {
    let mut stream = UnixStream::connect("/tmp/cni-ipam-consul.sock")?;
    stream.write_all(serde_json::to_string(&req)?.as_bytes())?;

    Ok(())
}

fn get_request_from_env() -> CniRequest {
    CniRequest {
        command: std::env::var("CNI_COMMAND").expect("No CNI Command. Is CNI_COMMAND set?"),
        container_id: std::env::var("CNI_CONTAINERID")
            .expect("No container ID. Is CNI_CONTAINER_ID set?"),
        netns: std::env::var("CNI_NETNS").expect("No nampesace set. Is CNI_NETNS set?"),
        ifname: std::env::var("CNI_IFNAME").expect("No interface name set. Is CNI_IFNAME set?"),
        args: std::env::var("CNI_ARGS").ok(),
        path: std::env::var("CNI_PATH").expect("No path set. Is CNI_PATH set?"),
    }
}
