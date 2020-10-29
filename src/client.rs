use crate::{CniRequest, IpamResponse};
use anyhow::{anyhow, Error, Result};
use serde_json;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;

pub fn run_client() -> Result<()> {
    let req = get_request_from_env();

    let resp = send_request(req)?;

    let resp_json = serde_json::to_string(&resp)?;

    println!("{}", resp_json);

    Ok(())
}

pub fn send_request(req: CniRequest) -> Result<IpamResponse> {
    let mut stream = UnixStream::connect("/tmp/cni-ipam-consul.sock")?;

    stream.write_all(serde_json::to_string(&req)?.as_bytes())?;
    stream.write_all("\n".as_bytes())?;

    let mut reader = BufReader::new(stream);

    let mut buf: String = String::new();
    let _ = reader.read_line(&mut buf)?;
    let resp: IpamResponse = serde_json::from_str(buf.as_str())?;

    Ok(resp)
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
