use cliplink_lib::{
    app_state::AppState,
    clipboard::{ClipboardError, ClipboardWriter},
    commands::{app_status, set_auto_write_remote_text},
};

#[derive(Default)]
struct RecordingClipboard;

impl ClipboardWriter for RecordingClipboard {
    fn write_text(&mut self, _text: &str) -> Result<(), ClipboardError> {
        Ok(())
    }

    fn write_png(&mut self, _png: &[u8]) -> Result<(), ClipboardError> {
        Ok(())
    }
}

#[test]
fn status_reports_auto_write_toggle() {
    let mut state = AppState::new(RecordingClipboard);

    assert!(!app_status(&state).auto_write_remote_text);

    let status = set_auto_write_remote_text(&mut state, true);

    assert!(status.auto_write_remote_text);
    assert!(app_status(&state).auto_write_remote_text);
}
