//! Implementation of the game manager.

use crate::game::game_session::GameSession;
use crate::game::player::{Player, Registry};
use tokio::sync::broadcast::Receiver;
use uuid::Uuid;

const MAX_PLAYERS_PER_SESSION: usize = 3;

pub struct GameManager {
    pub sessions: Vec<GameSession>,
}

impl GameManager {
    pub fn new() -> GameManager {
        Self {
            sessions: Vec::new(),
        }
    }

    pub fn get_session(&self, id: Uuid) -> Option<&GameSession> {
        self.sessions.iter().find(|s| s.id == id)
    }

    // Ok := party's id
    // Err := error
    pub fn register_player_on_party(
        &mut self,
        player: Player,
    ) -> Result<(Uuid, Receiver<String>), String> {
        // Checks for a party that isn't filled
        let avaliable_session = self
            .sessions
            .iter_mut()
            .find(|session| session.party.len() < MAX_PLAYERS_PER_SESSION);

        match avaliable_session {
            // Needs to create a new party
            None => {
                self.sessions.push(GameSession::new());

                if let Some(new_session) = self.sessions.last_mut() {
                    return match new_session.party.add(player) {
                        Ok(_) => {
                            let receiver = new_session.party.sender.subscribe();
                            Ok((new_session.id, receiver))
                        }
                        Err(err) => Err(err),
                    };
                }
                Err("Not enough sessions".to_string())
            }
            // There is at least 1 available and tries to insert on it
            Some(session) => match session.party.add(player) {
                Ok(_) => {
                    let receiver = session.party.sender.subscribe();
                    return Ok((session.id, receiver));
                }
                Err(err) => Err(err),
            },
        }
    }

    // TODO: it's removing from ALL sessions. Should remove only from 1, but it's papo for later
    pub fn remove_player_on_party(&mut self, username: &String) {
        self.sessions
            .retain(|session| !session.party.contains(username))
    }
}
