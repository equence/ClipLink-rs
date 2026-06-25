use cliplink_lib::{
    protocol::{decode, encode, Frame},
    relay::Relay,
};
use std::time::Duration;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    time::timeout,
};

#[tokio::test]
async fn relay_forwards_a_frame_to_other_connected_clients() {
    let relay = Relay::start("127.0.0.1:0".parse().expect("valid socket address"))
        .await
        .expect("relay starts");
    let address = relay.local_addr();
    let mut sender = TcpStream::connect(address).await.expect("sender connects");
    let mut receiver = TcpStream::connect(address)
        .await
        .expect("receiver connects");

    tokio::time::sleep(Duration::from_millis(20)).await;
    let encoded = encode(&Frame::from_text("relay message")).expect("frame encodes");
    sender.write_all(&encoded).await.expect("sender writes");

    let mut received = vec![0; encoded.len()];
    timeout(Duration::from_secs(1), receiver.read_exact(&mut received))
        .await
        .expect("receiver gets a frame")
        .expect("relay keeps receiver connected");

    assert_eq!(
        decode(&received)
            .expect("forwarded frame decodes")
            .text()
            .expect("text"),
        "relay message"
    );
    let mut unexpected = [0; 1];
    assert!(
        timeout(Duration::from_millis(50), sender.read(&mut unexpected))
            .await
            .is_err()
    );
}
