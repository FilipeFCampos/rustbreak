use std::io::Write;
use std::net::TcpStream;
use parking_lot::Mutex;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct Player {
    pub id: Uuid,
    pub connection: Arc<Mutex<TcpStream>>,
}

impl Player {
    pub fn new(stream: TcpStream) -> Self {
        Self {
            id: Uuid::new_v4(),
            connection: Arc::new(Mutex::new(stream)),
        }
    }
}

pub struct Registry {
    players: Vec<Player>,
}

impl Registry {
    pub fn new() -> Self {
        Self { players: Vec::new() }
    }

    pub fn add(&mut self, player: Player) -> Uuid {
        let id = player.id;
        self.players.push(player);
        id
    }

    pub fn remove(&mut self, id: Uuid) {
        self.players.retain(|p| p.id != id);
    }

    pub fn broadcast(&self, msg: &str) {
        let bytes = msg.as_bytes();
        for p in &self.players {
            let mut conn = p.connection.lock();
            let _ = conn.write(bytes);
        }
    }
}
