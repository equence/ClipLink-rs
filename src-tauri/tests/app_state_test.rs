use cliplink_lib::{
    app_state::AppState,
    clipboard::{ClipboardError, ClipboardWriter, SyncAction},
    connection::ConnectionEvent,
    protocol::Frame,
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
fn remote_text_frame_updates_status_and_clipboard_policy() {
    let mut state = AppState::new(RecordingClipboard::default());
    state.set_auto_write_remote_text(true);

    let action = state
        .handle_connection_event(ConnectionEvent::Frame(Frame::from_text("shared text")))
        .expect("event handled");

    assert_eq!(action, Some(SyncAction::TextWritten));
    assert_eq!(
        state.status().last_remote_text.as_deref(),
        Some("shared text")
    );
    assert_eq!(state.clipboard().texts, ["shared text"]);
}

#[test]
fn remote_image_frame_updates_cached_image_count_without_clipboard_write() {
    let mut state = AppState::new(RecordingClipboard::default());

    let action = state
        .handle_connection_event(ConnectionEvent::Frame(Frame::from_png(vec![1, 2, 3])))
        .expect("event handled");

    assert!(matches!(action, Some(SyncAction::ImageCached { .. })));
    assert_eq!(state.status().cached_image_count, 1);
    assert!(state.clipboard().pngs.is_empty());
}

#[test]
fn non_frame_connection_events_do_not_touch_clipboard() {
    let mut state = AppState::new(RecordingClipboard::default());

    let action = state
        .handle_connection_event(ConnectionEvent::Connected)
        .expect("event handled");

    assert_eq!(action, None);
    assert!(state.clipboard().texts.is_empty());
    assert_eq!(state.status().cached_image_count, 0);
}
