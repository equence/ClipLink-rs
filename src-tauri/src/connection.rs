use crate::protocol::{encode, try_decode, Frame};
use std::{fmt, net::SocketAddr, sync::Arc};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{tcp::OwnedWriteHalf, TcpStream},
    sync::{broadcast, Mutex},
};

#[derive(Clone, Debug)]
pub enum ConnectionEvent {
    Connected,
    Disconnected { reason: String },
    Frame(Frame),
}

#[derive(Clone)]
pub struct ConnectionManager {
    writer: Arc<Mutex<Option<OwnedWriteHalf>>>,
    events: broadcast::Sender<ConnectionEvent>,
}

impl ConnectionManager {
    pub fn new() -> Self {
        let (events, _) = broadcast::channel(64);
        Self { writer: Arc::new(Mutex::new(None)), events }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<ConnectionEvent> {
        self.events.subscribe()
    }

    pub async fn connect(&self, address: SocketAddr) -> Result<(), ConnectionError> {
        let stream = TcpStream::connect(address).await?;
        let (reader, writer) = stream.into_split();
        *self.writer.lock().await = Some(writer);
        let _ = self.events.send(ConnectionEvent::Connected);

        let events = self.events.clone();
        tokio::spawn(async move {
            let mut reader = reader;
            let mut read_buffer = [0; 8192];
            let mut frame_buffer = Vec::new();
            let reason = 'read_loop: loop {
                match reader.read(&mut read_buffer).await {
                    Ok(0) => break "server closed the connection".to_owned(),
                    Ok(read) => {
                        frame_buffer.extend_from_slice(&read_buffer[..read]);
                        loop {
                            match try_decode(&mut frame_buffer) {
                                Ok(Some(frame)) => { let _ = events.send(ConnectionEvent::Frame(frame)); }
                                Ok(None) => break,
                                Err(error) => break 'read_loop format!("invalid relay frame: {error}"),
                            }
                        }
                    }
                    Err(error) => break error.to_string(),
                }
            };
            let _ = events.send(ConnectionEvent::Disconnected { reason });
        });
        Ok(())
    }

    pub async fn send(&self, frame: &Frame) -> Result<(), ConnectionError> {
        let bytes = encode(frame)?;
        let mut writer = self.writer.lock().await;
        let writer = writer.as_mut().ok_or(ConnectionError::NotConnected)?;
        writer.write_all(&bytes).await?;
        Ok(())
    }
}

#[derive(Debug)]
pub enum ConnectionError {
    Io(std::io::Error),
    Protocol(crate::protocol::FrameError),
    NotConnected,
}

impl From<std::io::Error> for ConnectionError {
    fn from(error: std::io::Error) -> Self { Self::Io(error) }
}

impl From<crate::protocol::FrameError> for ConnectionError {
    fn from(error: crate::protocol::FrameError) -> Self { Self::Protocol(error) }
}

impl fmt::Display for ConnectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result { write!(formatter, "{self:?}") }
}

impl std::error::Error for ConnectionError {}
