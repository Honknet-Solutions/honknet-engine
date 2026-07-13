use std::sync::Arc;

use anyhow::Result;
use honknet_protocol::ServerMessage;
use tokio::sync::{broadcast, RwLock};

use crate::server_state::ServerState;

#[derive(Clone)]
pub struct AppState {
    pub game: Arc<RwLock<ServerState>>,
    pub events: broadcast::Sender<ServerMessage>,
}

impl AppState {
    pub fn new() -> Result<Self> {
        let (events, _) = broadcast::channel(128);
        Ok(Self {
            game: Arc::new(RwLock::new(ServerState::new_debug()?)),
            events,
        })
    }
}
