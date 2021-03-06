mod allocator;
mod client;
mod cni;
mod server;

use crate::client::run_client;
use crate::server::ConsulIpamServer;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::net::Ipv4Addr;

fn main() -> Result<()> {
    let server_arg = std::env::args().nth(1);
    if let Some(arg) = server_arg {
        if arg.to_lowercase() == "server" {
            ConsulIpamServer::new()?.run()?;
        } else {
            panic!("Got unexpected argument {}", arg);
        }
    }

    run_client()
}
