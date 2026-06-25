use cliplink_lib::{
    clipboard::{ClipboardError, ClipboardSync, ClipboardWriter, SyncAction},
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
fn remote_text_is_written_when_auto_write_is_enabled() {
    let mut sync = ClipboardSync::new(RecordingClipboard::default());
    sync.set_auto_write_remote_text(true);

    let action = sync
        .handle_remote_frame(&Frame::from_text("hello from peer"))
        .expect("remote text is handled");

    assert_eq!(action, SyncAction::TextWritten);
    assert_eq!(sync.clipboard().texts, ["hello from peer"]);
}

#[test]
fn remote_text_is_not_written_when_auto_write_is_disabled() {
    let mut sync = ClipboardSync::new(RecordingClipboard::default());
    sync.set_auto_write_remote_text(false);

    let action = sync
        .handle_remote_frame(&Frame::from_text("preview only"))
        .expect("remote text is handled");

    assert_eq!(action, SyncAction::TextSkipped);
    assert!(sync.clipboard().texts.is_empty());
}

#[test]
fn remote_png_is_cached_and_only_copied_on_request() {
    let mut sync = ClipboardSync::new(RecordingClipboard::default());
    sync.set_auto_write_remote_text(true);
    let png = vec![137, 80, 78, 71, 13, 10, 26, 10];

    let action = sync
        .handle_remote_frame(&Frame::from_png(png.clone()))
        .expect("remote png is handled");

    let SyncAction::ImageCached { id } = action else {
        panic!("expected image cached action");
    };
    assert_eq!(sync.cached_images()[0].bytes, png);
    assert!(sync.clipboard().pngs.is_empty());

    sync.copy_cached_image_to_clipboard(id)
        .expect("cached image can be copied");

    assert_eq!(sync.clipboard().pngs, [png]);
}
