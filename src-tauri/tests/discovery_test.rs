use cliplink_lib::discovery::{
    peer_from_service_info, service_info_for_relay, CLIPLINK_SERVICE_TYPE,
};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

#[test]
fn relay_service_info_uses_cliplink_service_type_and_version_txt() {
    let info = service_info_for_relay(
        "ClipLink Test",
        "cliplink-test.local.",
        IpAddr::V4(Ipv4Addr::new(192, 168, 1, 44)),
        41245,
    )
    .expect("service info builds");

    assert_eq!(info.get_type(), CLIPLINK_SERVICE_TYPE);
    assert_eq!(info.get_port(), 41245);
    assert_eq!(info.get_property_val_str("version"), Some("1"));
}

#[test]
fn service_info_converts_to_discovery_peer() {
    let info = service_info_for_relay(
        "ClipLink Test",
        "cliplink-test.local.",
        IpAddr::V4(Ipv4Addr::new(192, 168, 1, 44)),
        41245,
    )
    .expect("service info builds");

    let peer = peer_from_service_info(&info).expect("peer converts");

    assert_eq!(peer.name, "ClipLink Test");
    assert_eq!(peer.address, SocketAddr::from(([192, 168, 1, 44], 41245)));
    assert_eq!(peer.version.as_deref(), Some("1"));
}
