use crate::{
    clipboard::{ClipboardError, ClipboardSync, ClipboardWriter, SyncAction},
    connection::ConnectionEvent,
    protocol::MessageKind,
};

pub struct AppState<C> {
    clipboard_sync: ClipboardSync<C>,
    last_remote_text: Option<String>,
}

impl<C: ClipboardWriter> AppState<C> {
    pub fn new(clipboard: C) -> Self {
        Self {
            clipboard_sync: ClipboardSync::new(clipboard),
            last_remote_text: None,
        }
    }

    pub fn set_auto_write_remote_text(&mut self, enabled: bool) {
        self.clipboard_sync.set_auto_write_remote_text(enabled);
    }

    pub fn auto_write_remote_text(&self) -> bool {
        self.clipboard_sync.auto_write_remote_text()
    }

    pub fn handle_connection_event(
        &mut self,
        event: ConnectionEvent,
    ) -> Result<Option<SyncAction>, ClipboardError> {
        let ConnectionEvent::Frame(frame) = event else {
            return Ok(None);
        };

        if frame.kind() == MessageKind::Text {
            self.last_remote_text = Some(frame.text()?.to_owned());
        }

        self.clipboard_sync.handle_remote_frame(&frame).map(Some)
    }

    pub fn status(&self) -> AppStatus {
        AppStatus {
            auto_write_remote_text: self.auto_write_remote_text(),
            last_remote_text: self.last_remote_text.clone(),
            cached_image_count: self.clipboard_sync.cached_images().len(),
        }
    }

    pub fn clipboard(&self) -> &C {
        self.clipboard_sync.clipboard()
    }

    pub fn copy_cached_image_to_clipboard(&mut self, id: uuid::Uuid) -> Result<(), ClipboardError> {
        self.clipboard_sync.copy_cached_image_to_clipboard(id)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppStatus {
    pub auto_write_remote_text: bool,
    pub last_remote_text: Option<String>,
    pub cached_image_count: usize,
}
