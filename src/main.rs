mod client;
mod server;

use crate::client::run_client;
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CniRequest {
    command: String,
    container_id: String,
    netns: String,
    ifname: String,
    args: Option<String>,
    path: String,
}

fn main() -> Result<()> {
    let server_arg = std::env::args().nth(1);
    if let Some(arg) = server_arg {
        if arg.to_lowercase() == "server" {
            server::run_server()?;
        } else {
            panic!("Got unexpected argument {}", arg);
        }
    }

    run_client()
}
