use std::collections::HashMap;
use uuid::Uuid;

/// Models a player connected to the server.
#[derive(Clone)]
pub struct Player {
    pub id: Uuid,
    pub username: String,
}

impl Player {
    pub fn new(username: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            username,
        }
    }
}

/// Stores a `HashMap` of all players connected to the server.
///
/// Each player in the map is identified by its `username`. I used `username`
/// instead of `id` as the key to easily check if there are duplicate usernames
/// and prompt the client for a new one.
pub struct Registry {
    players: HashMap<String, Player>,
}

impl Registry {
    pub fn new() -> Self {
        Self {
            players: HashMap::new(),
        }
    }

    pub fn add(&mut self, player: Player) -> Option<Player> {
        self.players.insert(player.username.clone(), player)
    }

    pub fn remove(&mut self, username: String) -> Option<Player> {
        self.players.remove(&username)
    }
}
