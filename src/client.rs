use crate::cni::{CniRequest, IpamResponse};
use anyhow::{anyhow, Error, Result};
use log::{debug, error, info, trace, warn, LevelFilter, SetLoggerError};
use log4rs::append::file::FileAppender;
use log4rs::config::{Appender, Config, Logger, Root};
use log4rs::encode::pattern::PatternEncoder;
use log4rs::filter::threshold::ThresholdFilter;
use serde_json;
use std::io;
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::net::UnixStream;

pub fn run_client() -> Result<()> {
    init_logging();

    let req = get_request()?;

    info!("Handling request: {:?}", req);

    let resp = send_request(req)?;

    info!("Got response: {:?}", resp);

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

fn get_request() -> Result<CniRequest> {
    let mut stdin = io::stdin();
    let config = serde_json::from_reader(stdin)?;

    info!("CNI Config: {:?}", config);

    Ok(CniRequest {
        command: std::env::var("CNI_COMMAND").expect("No CNI Command. Is CNI_COMMAND set?"),
        container_id: std::env::var("CNI_CONTAINERID")
            .expect("No container ID. Is CNI_CONTAINER_ID set?"),
        netns: std::env::var("CNI_NETNS").expect("No nampesace set. Is CNI_NETNS set?"),
        ifname: std::env::var("CNI_IFNAME").expect("No interface name set. Is CNI_IFNAME set?"),
        args: std::env::var("CNI_ARGS").ok(),
        path: std::env::var("CNI_PATH").expect("No path set. Is CNI_PATH set?"),
        config,
    })
}

fn init_logging() {
    let logfile = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new("{d} - {l} - {L} - {m}{n}")))
        .build("/tmp/log/consul-ipam-client.log")
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
