use cliplink_lib::{
    connection::{ConnectionEvent, ConnectionManager},
    protocol::{encode, Frame},
    relay::Relay,
};
use std::{net::TcpListener as StdTcpListener, time::Duration};
use tokio::{io::AsyncWriteExt, net::TcpStream, time::timeout};

#[tokio::test]
async fn manager_emits_a_received_frame_from_the_relay() {
    let relay = Relay::start("127.0.0.1:0".parse().expect("valid address"))
        .await
        .expect("relay starts");
    let mut sender = TcpStream::connect(relay.local_addr())
        .await
        .expect("sender connects");
    let manager = ConnectionManager::new();
    let mut events = manager.subscribe();
    manager
        .connect(relay.local_addr())
        .await
        .expect("manager connects");

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

#[tokio::test]
async fn manager_ignores_duplicate_frames_from_the_relay() {
    let relay = Relay::start("127.0.0.1:0".parse().expect("valid address"))
        .await
        .expect("relay starts");
    let mut sender = TcpStream::connect(relay.local_addr())
        .await
        .expect("sender connects");
    let manager = ConnectionManager::new();
    let mut events = manager.subscribe();
    manager
        .connect(relay.local_addr())
        .await
        .expect("manager connects");

    tokio::time::sleep(Duration::from_millis(20)).await;
    let encoded = encode(&Frame::from_text("same message")).expect("frame encodes");
    sender
        .write_all(&encoded)
        .await
        .expect("sender writes first copy");
    sender
        .write_all(&encoded)
        .await
        .expect("sender writes duplicate copy");

    let mut received_frames = 0;
    loop {
        match timeout(Duration::from_millis(150), events.recv()).await {
            Ok(Ok(ConnectionEvent::Frame(frame))) => {
                assert_eq!(frame.text().expect("text payload"), "same message");
                received_frames += 1;
            }
            Ok(Ok(_)) => {}
            Ok(Err(error)) => panic!("event channel stays open: {error}"),
            Err(_) => break,
        }
    }

    assert_eq!(received_frames, 1);
}

#[tokio::test]
async fn manager_retries_initial_connect_until_relay_is_available() {
    let socket = StdTcpListener::bind("127.0.0.1:0").expect("finds an available port");
    let address = socket.local_addr().expect("local address is readable");
    drop(socket);

    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(50)).await;
        let relay = Relay::start(address).await.expect("delayed relay starts");
        tokio::time::sleep(Duration::from_millis(250)).await;
        drop(relay);
    });

    let manager = ConnectionManager::new();
    let mut events = manager.subscribe();
    manager
        .connect_with_retry(address, 5, Duration::from_millis(25))
        .await
        .expect("manager eventually connects");

    loop {
        let event = timeout(Duration::from_secs(1), events.recv())
            .await
            .expect("manager emits an event")
            .expect("event channel stays open");
        if matches!(event, ConnectionEvent::Connected) {
            break;
        }
    }
}
