use uuid::Uuid;

/// Models a player connected to the server.
#[derive(Clone, Hash, Eq, PartialEq)]
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

/*
/// Stores a `HashSet` of all players connected to a party and a broadcast channel
/// to allow communication between players.
///
/// Each player in the map is identified by its `username`, making easier check for duplicated users.
pub struct Registry {
    pub players: HashSet<Player>,
    // pub sender: broadcast::Sender<String>,
}

impl Registry {
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel::<String>(128);
        Self {
            players: HashSet::new(),
            sender,
        }
    }

    pub fn add(&mut self, player: Player) -> Result<(), String> {
        if self.contains(&player.username) {
            return Err(format!("Player already registered: {}", player.username));
        }

        match self.players.insert(player.clone()) {
            true => Ok(()),
            false => Err("Cannot add player twice".to_string()),
        }
    }

    pub fn remove(&mut self, username: String) {
        self.players.retain(|p| p.username != username)
    }

    pub fn len(&self) -> usize {
        self.players.len()
    }

    pub fn contains(&self, username: &String) -> bool {
        self.players
            .iter()
            .find(|p| p.username == *username)
            .is_some()
    }
}
*/
