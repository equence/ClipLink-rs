use crate::protocol::{encode, try_decode};
use std::{collections::HashMap, fmt, net::SocketAddr, sync::{atomic::{AtomicU64, Ordering}, Arc}};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{tcp::{OwnedReadHalf, OwnedWriteHalf}, TcpListener},
    sync::Mutex,
    task::JoinHandle,
};

type Peers = Arc<Mutex<HashMap<u64, OwnedWriteHalf>>>;

pub struct Relay {
    local_addr: SocketAddr,
    accept_task: JoinHandle<()>,
}

impl Relay {
    pub async fn start(bind: SocketAddr) -> Result<Self, RelayError> {
        let listener = TcpListener::bind(bind).await?;
        let local_addr = listener.local_addr()?;
        let peers = Arc::new(Mutex::new(HashMap::new()));
        let next_peer_id = Arc::new(AtomicU64::new(1));
        let accept_task = tokio::spawn(accept_connections(listener, peers, next_peer_id));

        Ok(Self { local_addr, accept_task })
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }
}

impl Drop for Relay {
    fn drop(&mut self) {
        self.accept_task.abort();
    }
}

#[derive(Debug)]
pub enum RelayError {
    Io(std::io::Error),
}

impl From<std::io::Error> for RelayError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl fmt::Display for RelayError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for RelayError {}

async fn accept_connections(listener: TcpListener, peers: Peers, next_peer_id: Arc<AtomicU64>) {
    while let Ok((stream, _)) = listener.accept().await {
        let peer_id = next_peer_id.fetch_add(1, Ordering::Relaxed);
        let (reader, writer) = stream.into_split();
        peers.lock().await.insert(peer_id, writer);
        tokio::spawn(read_frames(peer_id, reader, Arc::clone(&peers)));
    }
}

async fn read_frames(peer_id: u64, mut reader: OwnedReadHalf, peers: Peers) {
    let mut read_buffer = [0; 8192];
    let mut frame_buffer = Vec::new();

    loop {
        let read = match reader.read(&mut read_buffer).await {
            Ok(0) | Err(_) => break,
            Ok(read) => read,
        };
        frame_buffer.extend_from_slice(&read_buffer[..read]);

        loop {
            let frame = match try_decode(&mut frame_buffer) {
                Ok(Some(frame)) => frame,
                Ok(None) => break,
                Err(_) => return,
            };
            let encoded = match encode(&frame) {
                Ok(encoded) => encoded,
                Err(_) => return,
            };
            broadcast(peer_id, &encoded, &peers).await;
        }
    }

    peers.lock().await.remove(&peer_id);
}

async fn broadcast(sender_id: u64, frame: &[u8], peers: &Peers) {
    let mut peers = peers.lock().await;
    let mut disconnected = Vec::new();
    for (&peer_id, writer) in peers.iter_mut() {
        if peer_id != sender_id && writer.write_all(frame).await.is_err() {
            disconnected.push(peer_id);
        }
    }
    for peer_id in disconnected {
        peers.remove(&peer_id);
    }
}
