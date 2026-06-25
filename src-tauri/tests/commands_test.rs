use cliplink_lib::{
    app_state::AppState,
    clipboard::{ClipboardError, ClipboardWriter},
    commands::{app_status, set_auto_write_remote_text, CommandRuntime},
    connection::ConnectionManager,
    protocol::{try_decode, Frame},
    relay::Relay,
};
use std::{net::SocketAddr, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    time::timeout,
};

#[derive(Default)]
struct RecordingClipboard {
    texts: Vec<String>,
    pngs: Vec<Vec<u8>>,
}

impl ClipboardWriter for RecordingClipboard {
    fn write_text(&mut self, text: &str) -> Result<(), ClipboardError> {
        self.texts.push(text.to_owned());
        Ok(())
    }

    fn write_png(&mut self, png: &[u8]) -> Result<(), ClipboardError> {
        self.pngs.push(png.to_vec());
        Ok(())
    }
}

#[test]
fn status_reports_auto_write_toggle() {
    let mut state = AppState::new(RecordingClipboard::default());

    assert!(!app_status(&state).auto_write_remote_text);

    let status = set_auto_write_remote_text(&mut state, true);

    assert!(status.auto_write_remote_text);
    assert!(app_status(&state).auto_write_remote_text);
}

#[tokio::test]
async fn runtime_connects_to_relay_and_sends_text() {
    let relay = Relay::start("127.0.0.1:0".parse().expect("valid address"))
        .await
        .expect("relay starts");
    let mut receiver = TcpStream::connect(relay.local_addr())
        .await
        .expect("receiver connects");
    let runtime = CommandRuntime::new(RecordingClipboard::default(), ConnectionManager::new());

    runtime
        .connect_relay(relay.local_addr(), 3, Duration::from_millis(10))
        .await
        .expect("runtime connects");
    tokio::time::sleep(Duration::from_millis(20)).await;
    runtime
        .send_text("hello command")
        .await
        .expect("runtime sends text");

    let mut read_buffer = [0; 8192];
    let read = timeout(Duration::from_secs(1), receiver.read(&mut read_buffer))
        .await
        .expect("relay forwards a frame")
        .expect("receiver reads frame");
    let mut frame_buffer = read_buffer[..read].to_vec();
    let frame = try_decode(&mut frame_buffer)
        .expect("frame decodes")
        .expect("complete frame");
    assert_eq!(frame.text().expect("text payload"), "hello command");
}

#[tokio::test]
async fn runtime_applies_remote_frames_from_connection_events() {
    let relay = Relay::start("127.0.0.1:0".parse().expect("valid address"))
        .await
        .expect("relay starts");
    let mut sender = TcpStream::connect(relay.local_addr())
        .await
        .expect("sender connects");
    let mut runtime = CommandRuntime::new(RecordingClipboard::default(), ConnectionManager::new());
    runtime.set_auto_write_remote_text(true);

    runtime
        .connect_relay(relay.local_addr(), 3, Duration::from_millis(10))
        .await
        .expect("runtime connects");
    tokio::time::sleep(Duration::from_millis(20)).await;
    sender
        .write_all(
            &cliplink_lib::protocol::encode(&Frame::from_text("remote update"))
                .expect("frame encodes"),
        )
        .await
        .expect("sender writes");

    let status = timeout(Duration::from_secs(1), async {
        loop {
            let status = runtime.status();
            if status.last_remote_text.as_deref() == Some("remote update") {
                break status;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("runtime applies remote frame");

    assert_eq!(status.last_remote_text.as_deref(), Some("remote update"));
    assert_eq!(
        runtime.with_clipboard(|clipboard| clipboard.texts.clone()),
        ["remote update"]
    );
}

#[tokio::test]
async fn runtime_starts_embedded_relay_for_peers() {
    let mut runtime = CommandRuntime::new(RecordingClipboard::default(), ConnectionManager::new());

    let relay_addr = runtime
        .start_relay("127.0.0.1:0".parse::<SocketAddr>().expect("valid bind"))
        .await
        .expect("runtime starts relay");

    let _peer = TcpStream::connect(relay_addr)
        .await
        .expect("peer connects to embedded relay");
}

#[test]
fn runtime_copies_cached_image_to_clipboard() {
    let mut runtime = CommandRuntime::new(RecordingClipboard::default(), ConnectionManager::new());
    let action = runtime
        .handle_remote_frame(Frame::from_png(vec![1, 2, 3]))
        .expect("image cached");
    let cliplink_lib::clipboard::SyncAction::ImageCached { id } = action else {
        panic!("expected cached image");
    };

    let status = runtime.copy_cached_image(id).expect("cached image copied");

    assert_eq!(status.cached_image_count, 1);
    assert_eq!(
        runtime.with_clipboard(|clipboard| clipboard.pngs.clone()),
        [vec![1, 2, 3]]
    );
}
