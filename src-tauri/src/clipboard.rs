use crate::protocol::{Frame, FrameError, MessageKind};
use std::fmt;
use uuid::Uuid;

pub trait ClipboardWriter {
    fn write_text(&mut self, text: &str) -> Result<(), ClipboardError>;
    fn write_png(&mut self, png: &[u8]) -> Result<(), ClipboardError>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CachedImage {
    pub id: Uuid,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SyncAction {
    TextWritten,
    TextSkipped,
    ImageCached { id: Uuid },
}

pub struct ClipboardSync<C> {
    clipboard: C,
    auto_write_remote_text: bool,
    cached_images: Vec<CachedImage>,
}

impl<C: ClipboardWriter> ClipboardSync<C> {
    pub fn new(clipboard: C) -> Self {
        Self {
            clipboard,
            auto_write_remote_text: false,
            cached_images: Vec::new(),
        }
    }

    pub fn set_auto_write_remote_text(&mut self, enabled: bool) {
        self.auto_write_remote_text = enabled;
    }

    pub fn handle_remote_frame(&mut self, frame: &Frame) -> Result<SyncAction, ClipboardError> {
        match frame.kind() {
            MessageKind::Text => {
                if self.auto_write_remote_text {
                    self.clipboard.write_text(frame.text()?)?;
                    Ok(SyncAction::TextWritten)
                } else {
                    Ok(SyncAction::TextSkipped)
                }
            }
            MessageKind::ImagePng => {
                let id = frame.id();
                self.cached_images.push(CachedImage {
                    id,
                    bytes: frame.payload().to_vec(),
                });
                Ok(SyncAction::ImageCached { id })
            }
            kind => Err(ClipboardError::UnsupportedFrame(kind)),
        }
    }

    pub fn copy_cached_image_to_clipboard(&mut self, id: Uuid) -> Result<(), ClipboardError> {
        let image = self
            .cached_images
            .iter()
            .find(|image| image.id == id)
            .ok_or(ClipboardError::MissingImage(id))?;
        self.clipboard.write_png(&image.bytes)
    }

    pub fn cached_images(&self) -> &[CachedImage] {
        &self.cached_images
    }

    pub fn clipboard(&self) -> &C {
        &self.clipboard
    }
}

#[derive(Debug)]
pub enum ClipboardError {
    Frame(FrameError),
    MissingImage(Uuid),
    UnsupportedFrame(MessageKind),
}

impl From<FrameError> for ClipboardError {
    fn from(error: FrameError) -> Self {
        Self::Frame(error)
    }
}

impl fmt::Display for ClipboardError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ClipboardError {}
