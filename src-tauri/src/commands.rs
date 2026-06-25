use crate::{
    app_state::{AppState, AppStatus},
    clipboard::{ClipboardError, ClipboardWriter, SyncAction},
    connection::{ConnectionError, ConnectionEvent, ConnectionManager},
    protocol::Frame,
    relay::{Relay, RelayError},
};
use serde::Serialize;
use std::{fmt, net::SocketAddr, time::Duration};
use uuid::Uuid;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppStatusDto {
    pub auto_write_remote_text: bool,
    pub last_remote_text: Option<String>,
    pub cached_image_count: usize,
}

pub fn app_status<C: ClipboardWriter>(state: &AppState<C>) -> AppStatusDto {
    state.status().into()
}

pub fn set_auto_write_remote_text<C: ClipboardWriter>(
    state: &mut AppState<C>,
    enabled: bool,
) -> AppStatusDto {
    state.set_auto_write_remote_text(enabled);
    state.status().into()
}

pub struct CommandRuntime<C> {
    app_state: AppState<C>,
    connection: ConnectionManager,
    relay: Option<Relay>,
}

impl<C: ClipboardWriter> CommandRuntime<C> {
    pub fn new(clipboard: C, connection: ConnectionManager) -> Self {
        Self {
            app_state: AppState::new(clipboard),
            connection,
            relay: None,
        }
    }

    pub fn status(&self) -> AppStatusDto {
        app_status(&self.app_state)
    }

    pub fn set_auto_write_remote_text(&mut self, enabled: bool) -> AppStatusDto {
        set_auto_write_remote_text(&mut self.app_state, enabled)
    }

    pub async fn connect_relay(
        &self,
        address: SocketAddr,
        retry_attempts: usize,
        retry_delay: Duration,
    ) -> Result<AppStatusDto, CommandError> {
        self.connection
            .connect_with_retry(address, retry_attempts, retry_delay)
            .await?;
        Ok(self.status())
    }

    pub async fn start_relay(&mut self, bind: SocketAddr) -> Result<SocketAddr, CommandError> {
        let relay = Relay::start(bind).await?;
        let local_addr = relay.local_addr();
        self.relay = Some(relay);
        Ok(local_addr)
    }

    pub async fn send_text(&self, text: impl Into<String>) -> Result<(), CommandError> {
        self.connection.send(&Frame::from_text(text)).await?;
        Ok(())
    }

    pub fn handle_remote_frame(&mut self, frame: Frame) -> Result<SyncAction, CommandError> {
        let action = self
            .app_state
            .handle_connection_event(ConnectionEvent::Frame(frame))?
            .ok_or(CommandError::FrameExpected)?;
        Ok(action)
    }

    pub fn copy_cached_image(&mut self, id: Uuid) -> Result<AppStatusDto, CommandError> {
        self.app_state.copy_cached_image_to_clipboard(id)?;
        Ok(self.status())
    }

    pub fn clipboard(&self) -> &C {
        self.app_state.clipboard()
    }
}

#[derive(Debug)]
pub enum CommandError {
    Clipboard(ClipboardError),
    Connection(ConnectionError),
    FrameExpected,
    Relay(RelayError),
}

impl From<ClipboardError> for CommandError {
    fn from(error: ClipboardError) -> Self {
        Self::Clipboard(error)
    }
}

impl From<ConnectionError> for CommandError {
    fn from(error: ConnectionError) -> Self {
        Self::Connection(error)
    }
}

impl From<RelayError> for CommandError {
    fn from(error: RelayError) -> Self {
        Self::Relay(error)
    }
}

impl fmt::Display for CommandError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for CommandError {}

impl From<AppStatus> for AppStatusDto {
    fn from(status: AppStatus) -> Self {
        Self {
            auto_write_remote_text: status.auto_write_remote_text,
            last_remote_text: status.last_remote_text,
            cached_image_count: status.cached_image_count,
        }
    }
}
