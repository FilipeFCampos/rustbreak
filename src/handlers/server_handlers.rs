use crate::game::game_session::{GameSession, MAX_PLAYERS_PER_SESSION};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::broadcast::{Receiver, Sender};
use tokio::sync::{Mutex, broadcast};
use uuid::Uuid;


pub type ServerSessions = Arc<Mutex<HashMap<Uuid, (GameSession, Sender<String>)>>>;

pub async fn register_player(
    sessions_arc: &ServerSessions,
    player: String,
) -> Result<(Uuid, Receiver<String>), String> {
    let mut sessions = sessions_arc.lock().await;
    let session_id: Uuid;
    let receiver: Receiver<String>;

    let avaliable_session = sessions
        .iter_mut()
        .find(|(_, (session, _))| session.party.len() < MAX_PLAYERS_PER_SESSION);

    let res: Result<(Uuid, Receiver<String>), String> = match avaliable_session {
        // Needs to create a new party
        None => {
            let (sender, _) = broadcast::channel::<String>(128);
            let game_session = GameSession::new();
            sessions.insert(game_session.id, (game_session, sender));

            if let Some((_, (new_session, sender))) = sessions.iter_mut().last() {
                match new_session.add_player(player) {
                    Ok(_) => {
                        session_id = new_session.id;
                        receiver = sender.subscribe();
                    }
                    Err(err) => return Err(err),
                }
                Ok((session_id, receiver))
            } else {
                Err("Not enough sessions".to_string())
            }
        }
        // There is at least 1 available and tries to insert on it
        Some((_, (session, sender))) => match session.add_player(player) {
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
            if let Some((_, (session, sender))) = sessions
                .iter_mut()
                .find(|(_, (session, _))| session.contains(username))
            {
                session.remove_player(username);
            }
        }
        Some(id) => {
            if let Some((session, _)) = sessions.get_mut(&id) {
                session.remove_player(username);
            }
        }
    }
    drop(sessions);
}
