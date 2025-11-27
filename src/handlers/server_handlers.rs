use crate::game::game_session::GameSession;
use std::sync::Arc;
use tokio::sync::broadcast::{Receiver, Sender};
use tokio::sync::{Mutex, broadcast};
use uuid::Uuid;

const MAX_PLAYERS_PER_SESSION: usize = 3;

pub type ServerSessions = Arc<Mutex<Vec<(GameSession, Sender<String>)>>>;

pub async fn register_player(
    sessions_arc: &ServerSessions,
    player: String,
) -> Result<(Uuid, Receiver<String>), String> {
    let mut sessions = sessions_arc.lock().await;
    let session_id: Uuid;
    let receiver: Receiver<String>;

    let avaliable_session = sessions
        .iter_mut()
        .find(|(session, _)| session.party.len() < MAX_PLAYERS_PER_SESSION);

    let res: Result<(Uuid, Receiver<String>), String> = match avaliable_session {
        // Needs to create a new party
        None => {
            let (sender, _) = broadcast::channel::<String>(128);
            sessions.push((GameSession::new(), sender));

            if let Some((new_session, sender)) = sessions.last_mut() {
                match new_session.add_player(player) {
                    Ok(_) => {
                        session_id = new_session.id;
                        receiver = sender.subscribe();
                        (session_id, receiver)
                    }
                    Err(err) => return Err(err),
                };
            }
            Err("Not enough sessions".to_string())
        }
        // There is at least 1 available and tries to insert on it
        Some((session, sender)) => match session.add_player(player) {
            Ok(_) => {
                session_id = session.id;
                receiver = sender.subscribe();
                Ok((session_id, receiver))
            }
            Err(err) => return Err(err),
        },
    };

    drop(sessions);
    res
}

pub async fn remove_player(
    sessions_arc: &ServerSessions,
    username: &String,
    party_id: Option<Uuid>,
) {
    let mut sessions = sessions_arc.lock().await;
    match party_id {
        None => {
            if let Some((session, sender)) = sessions
                .iter_mut()
                .find(|(session, _)| session.contains(username))
            {
                session.remove_player(username);
            }
        }
        Some(id) => {
            if let Some((session, _)) = sessions.iter_mut().find(|(s, _)| s.id == id) {
                session.remove_player(username);
            }
        }
    }
    drop(sessions);
}

