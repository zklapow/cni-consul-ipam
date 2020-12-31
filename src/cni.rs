use cidr::{Cidr, Ipv4Cidr, Ipv4Inet};
use serde::{Deserialize, Serialize, Serializer};
use std::collections::BTreeMap as Map;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CniConfig {
    #[serde(rename = "type")]
    pub plugin_type: String,
    pub name: String,
    #[serde(default)]
    pub args: Map<String, String>,
    pub ipam: ConsulIpamConfig,
    #[serde(skip_serializing_if = "Option::is_none")]
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
    pub path: Option<String>,
    pub subnet: Ipv4Cidr,
    pub gateway: Ipv4Inet,
    #[serde(default)]
    pub routes: Vec<Route>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CniRequest {
    pub command: String,
    pub container_id: String,
    pub netns: String,
    pub ifname: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<String>,
    pub path: String,
    pub config: CniConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpamResponse {
    pub cni_version: String,
    pub ips: Vec<IpResponse>,
    pub routes: Vec<Route>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dns: Option<DnsConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpResponse {
    pub version: String,
    #[serde(serialize_with = "serialize_host_ip")]
    pub address: Ipv4Cidr,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gateway: Option<Ipv4Inet>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interface: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Route {
    pub dst: Ipv4Cidr,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gw: Option<Ipv4Inet>,
}

impl IpamResponse {
    pub fn new(ips: Vec<IpResponse>, routes: Vec<Route>, dns: Option<DnsConfig>) -> IpamResponse {
        IpamResponse {
            cni_version: "v0.4.0".to_string(),
            ips,
            routes,
            dns,
        }
    }
}

fn serialize_host_ip<S>(addr: &Ipv4Cidr, ser: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    if addr.is_host_address() {
        ser.serialize_str(&format!("{}/{}", addr, 22))
    } else {
        ser.serialize_str(&format!("{}", addr))
    }
}
