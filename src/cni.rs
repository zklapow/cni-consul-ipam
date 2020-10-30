use cidr::{Ipv4Cidr, Ipv4Inet};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap as Map;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CniConfig {
    #[serde(rename = "type")]
    pub plugin_type: String,
    pub name: String,
    #[serde(default)]
    pub args: Map<String, String>,
    pub ipam: ConsulIpamConfig,
    pub dns: Option<DnsConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsConfig {
    #[serde(default)]
    pub nameservers: Vec<Ipv4Inet>,
    pub domain: Option<String>,
    #[serde(default)]
    pub search: Vec<String>,
    #[serde(default)]
    pub options: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsulIpamConfig {
    #[serde(rename = "type")]
    pub plugin_type: String,
    pub subnet: Ipv4Cidr,
    #[serde(default)]
    pub routes: Vec<Route>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CniRequest {
    pub command: String,
    pub container_id: String,
    pub netns: String,
    pub ifname: String,
    pub args: Option<String>,
    pub path: String,
    pub config: CniConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpamResponse {
    pub cni_version: String,
    pub ips: Vec<IpResponse>,
    pub routes: Vec<Route>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpResponse {
    pub version: String,
    pub address: Ipv4Inet,
    pub gateway: Option<Ipv4Inet>,
    pub interface: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Route {
    pub dst: Ipv4Cidr,
    pub gw: Option<Ipv4Inet>,
}

impl IpamResponse {
    pub fn new(ips: Vec<IpResponse>, routes: Vec<Route>) -> IpamResponse {
        IpamResponse {
            cni_version: "v0.4.0".to_string(),
            ips,
            routes,
        }
    }
}