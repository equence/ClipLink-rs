use crate::protocol::{encode, try_decode, Frame};
use std::{collections::HashSet, fmt, net::SocketAddr, sync::Arc, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{tcp::OwnedWriteHalf, TcpStream},
    sync::{broadcast, Mutex},
};
use uuid::Uuid;

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
    seen_frame_ids: Arc<Mutex<HashSet<Uuid>>>,
}

impl ConnectionManager {
    pub fn new() -> Self {
        let (events, _) = broadcast::channel(64);
        Self {
            writer: Arc::new(Mutex::new(None)),
            events,
            seen_frame_ids: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<ConnectionEvent> {
        self.events.subscribe()
    }

    pub async fn connect(&self, address: SocketAddr) -> Result<(), ConnectionError> {
        self.connect_once(address).await
    }

    pub async fn connect_with_retry(
        &self,
        address: SocketAddr,
        retry_attempts: usize,
        retry_delay: Duration,
    ) -> Result<(), ConnectionError> {
        let mut attempts = 0;
        loop {
            match self.connect_once(address).await {
                Ok(()) => return Ok(()),
                Err(error) => {
                    if attempts >= retry_attempts {
                        return Err(error);
                    }
                    attempts += 1;
                    tokio::time::sleep(retry_delay).await;
                }
            }
        }
    }

    async fn connect_once(&self, address: SocketAddr) -> Result<(), ConnectionError> {
        let stream = TcpStream::connect(address).await?;
        let (reader, writer) = stream.into_split();
        *self.writer.lock().await = Some(writer);
        let _ = self.events.send(ConnectionEvent::Connected);

        let events = self.events.clone();
        let seen_frame_ids = Arc::clone(&self.seen_frame_ids);
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
                                Ok(Some(frame)) => {
                                    if seen_frame_ids.lock().await.insert(frame.id()) {
                                        let _ = events.send(ConnectionEvent::Frame(frame));
                                    }
                                }
                                Ok(None) => break,
                                Err(error) => {
                                    break 'read_loop format!("invalid relay frame: {error}")
                                }
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
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<crate::protocol::FrameError> for ConnectionError {
    fn from(error: crate::protocol::FrameError) -> Self {
        Self::Protocol(error)
    }
}

impl fmt::Display for ConnectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ConnectionError {}
