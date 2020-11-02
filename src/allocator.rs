use anyhow::{anyhow, Context, Result};
use cidr::{Cidr, Inet, Ipv4Cidr, Ipv4Inet};
use clokwerk::timeprovider::ChronoTimeProvider;
use clokwerk::{Job, Scheduler, TimeUnits};
use consul::kv::{KVPair, KV};
use consul::session::{Session, SessionEntry};
use consul::{Client, Config};
use hostname;
use log::{error, info, warn};
use std::collections::BTreeMap as Map;
use std::convert::TryFrom;
use std::error::Error;
use std::net::Ipv4Addr;
use std::ops::Add;
use std::str::FromStr;
use std::thread::sleep;
use std::time::Duration;
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct ConsulIpAllocator {
    client: Client,
    session_id: String,
    lease_map: Map<String, Ipv4Addr>,
}

impl ConsulIpAllocator {
    pub fn new() -> Result<ConsulIpAllocator> {
        let config = Config::new().unwrap();
        let client = Client::new(config);

        let hostname = hostname::get()?;
        let hostname = hostname.to_str().expect("Could not get hostname");

        let entry = SessionEntry {
            Name: Some(String::from("consul-ipam-") + hostname),
            TTL: Some(String::from("30s")),
            Behavior: Some(String::from("delete")),
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
            lease_map: Map::new(),
        })
    }

    pub fn start(&self, scheduler: &mut Scheduler) {
        let renewal_clone = self.clone();
        scheduler.every(10.seconds()).run(move || {
            info!("Renewing consul session");
            renewal_clone
                .client
                .renew(renewal_clone.session_id.as_str(), None);
        });
    }

    pub fn stop(&self) {
        if let Some(e) = self.client.destroy(self.session_id.as_str(), None).err() {
            error!("Failed to close consul session: {}", e);
        }
    }

    pub fn allocate_from(
        &mut self,
        network_name: String,
        container_id: String,
        cidr: Ipv4Cidr,
    ) -> Result<Ipv4Addr> {
        let prefix = format!("ipam/{}/", network_name);

        let allocated_ips: Vec<Ipv4Addr> = KV::list(&self.client, prefix.as_str(), None)
            .map_err(|_| ConsulError::GetError)?
            .0
            .iter()
            .map(|pair| Ipv4Addr::from_str(pair.Value.as_str()).ok())
            .flatten()
            .collect();

        // Skip all IPs in the CIDR prior to last-allocated
        let mut ip_iter = cidr.iter().filter(|addr| {
            if *addr == cidr.first_address() {
                return false;
            } else if allocated_ips.contains(&addr) {
                return false;
            }

            return true;
        });

        // Get the next IP in the CIDR, wrapping back to the first IP (skipping .0) if we have reached the end
        let mut next_ip = ip_iter
            .next()
            .ok_or(anyhow!("No available IP in iterator"))?;

        let mut ip_key = format!("ipam/{}/{}", network_name, next_ip.to_string());

        while self
            .client
            .get(ip_key.as_str(), None)
            .ok()
            .map(|res| res.0)
            .flatten()
            .is_some()
        {
            next_ip = ip_iter
                .next()
                .ok_or(anyhow!("No available IP in iterator"))?;
            ip_key = format!("ipam/{}/{}", network_name, next_ip.to_string());
        }

        let alloc_kv_pair = KVPair {
            Key: ip_key,
            Value: container_id.clone(),
            Session: Some(self.session_id.clone()),
            ..Default::default()
        };

        self.client
            .acquire(&alloc_kv_pair, None)
            .map_err(|_| ConsulError::PutError)?;

        self.lease_map.insert(container_id.clone(), next_ip);

        Ok(next_ip)
    }

    pub fn release_from(&mut self, network_name: String, container_id: String) -> Result<()> {
        let leased_ip = self.lease_map.get(container_id.as_str());

        if let Some(addr) = leased_ip {
            info!(
                "Found leased IP {} for container {}, releasing",
                addr, container_id
            );

            let pair = KVPair {
                Key: format!("ipam/{}/{}", network_name, addr.to_string()),
                Value: container_id.clone(),
                Session: Some(self.session_id.clone()),
                ..Default::default()
            };

            self.client
                .release(&pair, None)
                .map_err(|e| ConsulError::LockError)?;

            self.client
                .delete(pair.Key.as_str(), None)
                .map_err(|e| ConsulError::LockError)?;
        }

        Ok(())
    }
}

#[derive(Error, Debug)]
pub enum ConsulError {
    #[error("Error acquiring lock")]
    LockError,
    #[error("Error getting key")]
    GetError,
    #[error("Error updating key")]
    PutError,
}
