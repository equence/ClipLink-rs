use cliplink_lib::{
    connection::{ConnectionEvent, ConnectionManager},
    protocol::{encode, Frame},
    relay::Relay,
};
use std::time::Duration;
use tokio::{io::AsyncWriteExt, net::TcpStream, time::timeout};

#[tokio::test]
async fn manager_emits_a_received_frame_from_the_relay() {
    let relay = Relay::start("127.0.0.1:0".parse().expect("valid address"))
        .await
        .expect("relay starts");
    let mut sender = TcpStream::connect(relay.local_addr()).await.expect("sender connects");
    let manager = ConnectionManager::new();
    let mut events = manager.subscribe();
    manager.connect(relay.local_addr()).await.expect("manager connects");

    tokio::time::sleep(Duration::from_millis(20)).await;
    sender
        .write_all(&encode(&Frame::from_text("from peer")).expect("frame encodes"))
        .await
        .expect("sender writes");

    loop {
        let event = timeout(Duration::from_secs(1), events.recv())
            .await
            .expect("manager emits an event")
            .expect("event channel stays open");
        if let ConnectionEvent::Frame(frame) = event {
            assert_eq!(frame.text().expect("text payload"), "from peer");
            break;
        }
    }
}
