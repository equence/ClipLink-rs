use mdns_sd::{Receiver, ServiceDaemon, ServiceEvent, ServiceInfo};
use serde::Serialize;
use std::{
    fmt,
    net::{IpAddr, SocketAddr},
};

pub const CLIPLINK_SERVICE_TYPE: &str = "_cliplink._tcp.local.";
pub const CLIPLINK_PROTOCOL_VERSION: &str = "1";

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveredPeer {
    pub id: String,
    pub name: String,
    pub host: String,
    pub address: SocketAddr,
    pub version: Option<String>,
}

pub struct LanDiscovery {
    daemon: ServiceDaemon,
}

impl LanDiscovery {
    pub fn new() -> Result<Self, DiscoveryError> {
        let daemon = ServiceDaemon::new().map_err(DiscoveryError::Mdns)?;
        Ok(Self { daemon })
    }

    pub fn publish_relay(
        &self,
        instance_name: &str,
        host_name: &str,
        ip: IpAddr,
        port: u16,
    ) -> Result<(), DiscoveryError> {
        let info = service_info_for_relay(instance_name, host_name, ip, port)?;
        self.daemon.register(info).map_err(DiscoveryError::Mdns)
    }

    pub fn browse(&self) -> Result<Receiver<ServiceEvent>, DiscoveryError> {
        self.daemon
            .browse(CLIPLINK_SERVICE_TYPE)
            .map_err(DiscoveryError::Mdns)
    }
}

pub fn service_info_for_relay(
    instance_name: &str,
    host_name: &str,
    ip: IpAddr,
    port: u16,
) -> Result<ServiceInfo, DiscoveryError> {
    let properties = [("version", CLIPLINK_PROTOCOL_VERSION)];
    ServiceInfo::new(
        CLIPLINK_SERVICE_TYPE,
        instance_name,
        host_name,
        ip,
        port,
        &properties[..],
    )
    .map_err(DiscoveryError::Mdns)
}

pub fn peer_from_service_info(info: &ServiceInfo) -> Option<DiscoveredPeer> {
    let ip = info.get_addresses().iter().next().copied()?;
    let address = SocketAddr::new(ip, info.get_port());
    Some(DiscoveredPeer {
        id: info.get_fullname().to_owned(),
        name: instance_name_from_fullname(info.get_fullname()),
        host: info.get_hostname().to_owned(),
        address,
        version: info.get_property_val_str("version").map(ToOwned::to_owned),
    })
}

fn instance_name_from_fullname(fullname: &str) -> String {
    fullname
        .strip_suffix(CLIPLINK_SERVICE_TYPE)
        .and_then(|name| name.strip_suffix('.'))
        .unwrap_or(fullname)
        .replace("\\.", ".")
}

#[derive(Debug)]
pub enum DiscoveryError {
    Mdns(mdns_sd::Error),
}

impl fmt::Display for DiscoveryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for DiscoveryError {}
