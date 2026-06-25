use crate::{
    app_state::{AppState, AppStatus},
    clipboard::ClipboardWriter,
};
use serde::Serialize;

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

impl From<AppStatus> for AppStatusDto {
    fn from(status: AppStatus) -> Self {
        Self {
            auto_write_remote_text: status.auto_write_remote_text,
            last_remote_text: status.last_remote_text,
            cached_image_count: status.cached_image_count,
        }
    }
}
