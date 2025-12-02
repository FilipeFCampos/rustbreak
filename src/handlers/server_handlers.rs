use crate::game::game_session::{GameEvent, GameSession};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use tokio::sync::{Mutex, broadcast, mpsc};
use uuid::Uuid;

// MUDANÇA IMPORTANTE:
// Antes: armazenava (GameSession, Sender)
// Agora: armazenamos (Arc<Mutex<GameSession>>, Sender)
// Por quê? Para permitir múltiplas tasks acessarem a mesma sessão simultaneamente.
pub struct SessionEntry {
    pub session: Arc<Mutex<GameSession>>,
    pub broadcast: broadcast::Sender<String>,
    pub event_channel: mpsc::Sender<GameEvent>,
}

impl SessionEntry {
    pub fn new(
        session: Arc<Mutex<GameSession>>,
        broadcast: broadcast::Sender<String>,
        event_channel: mpsc::Sender<GameEvent>,
    ) -> Self {
        Self {
            session,
            broadcast,
            event_channel,
        }
    }
}
pub type ServerSessions = Arc<Mutex<HashMap<Uuid, SessionEntry>>>;
