use sha2::{Digest, Sha256};
use std::fmt;
use uuid::Uuid;

const MAGIC: [u8; 4] = *b"CLNK";
const VERSION: u8 = 1;
const HEADER_LEN: usize = 4 + 1 + 1 + 16 + 16 + 4 + 32;
pub const MAX_PAYLOAD_BYTES: usize = 20 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum MessageKind {
    Text = 1,
    ImagePng = 2,
    Ping = 3,
}

impl TryFrom<u8> for MessageKind {
    type Error = FrameError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::Text),
            2 => Ok(Self::ImagePng),
            3 => Ok(Self::Ping),
            _ => Err(FrameError::UnknownMessageKind(value)),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Frame {
    id: Uuid,
    origin: Uuid,
    kind: MessageKind,
    payload: Vec<u8>,
}

impl Frame {
    pub fn from_text(text: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            origin: Uuid::new_v4(),
            kind: MessageKind::Text,
            payload: text.into().into_bytes(),
        }
    }

    pub fn kind(&self) -> MessageKind {
        self.kind
    }

    pub fn text(&self) -> Result<&str, FrameError> {
        if self.kind != MessageKind::Text {
            return Err(FrameError::NotText);
        }

        std::str::from_utf8(&self.payload).map_err(FrameError::InvalidUtf8)
    }
}

#[derive(Debug)]
pub enum FrameError {
    TooShort,
    InvalidMagic,
    UnsupportedVersion(u8),
    UnknownMessageKind(u8),
    PayloadTooLarge(usize),
    InvalidLength,
    HashMismatch,
    InvalidUtf8(std::str::Utf8Error),
    NotText,
}

impl fmt::Display for FrameError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for FrameError {}

pub fn encode(frame: &Frame) -> Result<Vec<u8>, FrameError> {
    if frame.payload.len() > MAX_PAYLOAD_BYTES {
        return Err(FrameError::PayloadTooLarge(frame.payload.len()));
    }

    let payload_len = u32::try_from(frame.payload.len()).map_err(|_| FrameError::PayloadTooLarge(frame.payload.len()))?;
    let mut bytes = Vec::with_capacity(HEADER_LEN + frame.payload.len());
    bytes.extend_from_slice(&MAGIC);
    bytes.push(VERSION);
    bytes.push(frame.kind as u8);
    bytes.extend_from_slice(frame.id.as_bytes());
    bytes.extend_from_slice(frame.origin.as_bytes());
    bytes.extend_from_slice(&payload_len.to_be_bytes());
    bytes.extend_from_slice(&Sha256::digest(&frame.payload));
    bytes.extend_from_slice(&frame.payload);
    Ok(bytes)
}

pub fn decode(bytes: &[u8]) -> Result<Frame, FrameError> {
    if bytes.len() < HEADER_LEN {
        return Err(FrameError::TooShort);
    }
    if bytes[..4] != MAGIC {
        return Err(FrameError::InvalidMagic);
    }
    if bytes[4] != VERSION {
        return Err(FrameError::UnsupportedVersion(bytes[4]));
    }

    let kind = MessageKind::try_from(bytes[5])?;
    let id = Uuid::from_slice(&bytes[6..22]).map_err(|_| FrameError::InvalidLength)?;
    let origin = Uuid::from_slice(&bytes[22..38]).map_err(|_| FrameError::InvalidLength)?;
    let payload_len = u32::from_be_bytes(bytes[38..42].try_into().expect("fixed header slice")) as usize;
    if payload_len > MAX_PAYLOAD_BYTES {
        return Err(FrameError::PayloadTooLarge(payload_len));
    }
    if bytes.len() != HEADER_LEN + payload_len {
        return Err(FrameError::InvalidLength);
    }

    let expected_hash = &bytes[42..74];
    let payload = bytes[HEADER_LEN..].to_vec();
    if Sha256::digest(&payload).as_slice() != expected_hash {
        return Err(FrameError::HashMismatch);
    }

    Ok(Frame { id, origin, kind, payload })
}
