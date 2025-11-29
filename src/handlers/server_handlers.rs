use crate::game::game_session::{GameEvent, GameSession};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, Mutex};
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

/*
// basicamente, quantos acertaram e quantos erraram, talvez a implementaçao nao é a melhor but it works
// assim, nao sei onde botar isso, entao coloquei aqui, deve ir no game_session.rs talvez
pub struct RoundResult {
    pub expected: usize, // quantos jogadores deveriam responder
    pub received: usize, // quantas respostas foram recebidas
    pub correct: usize, // quantas respostas corretas
    pub wrong: usize, // quantas respostas erradas
}

// NOVO: calcular maioria a partir de GameSession.answers
pub fn evaluate_round(session: &GameSession) -> RoundResult {
    let received = session.answers.len();
    let correct = session.answers.values().filter(|v| **v).count();
    let wrong = received - correct;

    RoundResult {
        expected: session.party.len(),
        received,
        correct,
        wrong,
    }
}
*/
