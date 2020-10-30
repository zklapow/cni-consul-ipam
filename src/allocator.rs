use anyhow::{anyhow, Result};
use cidr::{Ipv4Cidr, Ipv4Inet};
use clokwerk::timeprovider::ChronoTimeProvider;
use clokwerk::{Job, Scheduler, TimeUnits};
use consul::session::{Session, SessionEntry};
use consul::{Client, Config};
use hostname;
use std::str::FromStr;

#[derive(Debug, Clone)]
pub struct ConsulIpAllocator {
    client: Client,
    session_id: String,
}

impl ConsulIpAllocator {
    pub fn new() -> Result<ConsulIpAllocator> {
        let config = Config::new().unwrap();
        let client = Client::new(config);

        let hostname = hostname::get()?;
        let hostname = hostname.to_str().expect("Could not get hostname");

        let entry = SessionEntry {
            Name: Some(String::from("consul-ipam") + hostname),
            TTL: Some(String::from("2m")),
            ..Default::default()
        };

        let id: String = client
            .create(&entry, None)
            .map_err(|_| anyhow!("Could not create session"))?
            .0
            .ID
            .ok_or(anyhow!("No session ID"))?;

        Ok(ConsulIpAllocator {
            client,
            session_id: id,
        })
    }

    pub fn start(&self, scheduler: &mut Scheduler) {
        let renewal_clone = self.clone();
        scheduler.every(30.seconds()).run(move || {
            println!("Renewing consul session");
            renewal_clone
                .client
                .renew(renewal_clone.session_id.as_str(), None);
        });
    }

    pub fn allocate_from(cidr: Ipv4Cidr) -> Result<Ipv4Inet> {
        let ip = Ipv4Inet::from_str("192.168.1.1")?;
        Ok(ip)
    }
}
