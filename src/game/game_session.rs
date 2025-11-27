use crate::game::game_scene::{GameScene, GameSceneState};
use crate::game::player::Player;
use std::collections::HashSet;
use std::fs::{File, OpenOptions};
use std::io::BufReader;
use std::path::PathBuf;
use tokio::sync::broadcast::Sender;
use uuid::Uuid;

pub const MAX_PLAYERS_PER_SESSION: usize = 3;

#[derive(Clone)]
pub enum GameTurn {
    Server,
    Player,
}

pub enum GameEvent {}

#[derive(Clone)]
pub struct GameSession {
    pub id: Uuid,
    pub current_scene_state: GameSceneState,
    pub current_turn: GameTurn,
    pub party: HashSet<Player>,
}

impl GameSession {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            current_turn: GameTurn::Server,
            current_scene_state: GameSceneState::Prelude,
            party: HashSet::new(),
        }
    }

    pub fn add_player(&mut self, username: String) -> Result<(), String> {
        if self.contains(&username) {
            return Err(format!("Player already registered: {}", username));
        } else if self.party.len() >= 3 {
            return Err(format!("Party already full: {}", username));
        }

        match self.party.insert(Player::new(username.clone())) {
            true => Ok(()),
            false => Err("Cannot add player twice".to_string()),
        }
    }

    pub fn contains(&self, username: &String) -> bool {
        self.party
            .iter()
            .find(|p| p.username == *username)
            .is_some()
    }

    pub fn remove_player(&mut self, username: &String) {
        self.party.retain(|p| p.username != *username)
    }

    pub fn update(&mut self, event: GameEvent) {
        //TODO O PROXIMO PASSO SERIA AQUI AMANHA EU FAÇO ISSO :D
    }

    pub fn get_scene_json(&self) -> Option<String> {
        match &self.current_scene_state {
            GameSceneState::Normal(scene) => serde_json::to_string(scene).ok(),
            _ => None,
        }
    }

    fn load_scene(&mut self, path: &str) -> Result<(), &'static str> {
        //thought absolute paths were bad practice but this is the only way i could get it to work turns out it was an ally
        let mut full_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        full_path.push("data");
        full_path.push(path);

        match File::open(&full_path) {
            Ok(file) => {
                let reader = BufReader::new(file);
                match serde_json::from_reader::<BufReader<File>, GameScene>(reader) {
                    Ok(game_scene) => {
                        self.current_scene_state = GameSceneState::Normal(game_scene);
                        Ok(())
                    }
                    Err(_) => Err("Failed to deserialize game scene."),
                }
            }
            Err(_) => Err("Failed to load scene."),
        }
    }

    pub fn toggle_turn(&mut self) {
        match self.current_turn {
            GameTurn::Server => self.current_turn = GameTurn::Player,
            GameTurn::Player => self.current_turn = GameTurn::Server,
        }
    }

    pub fn begin_game(&mut self) {
        self.current_scene_state = GameSceneState::Prelude;
        if let Err(_) = self.load_scene("scene_1.json") {
            println!("Error loading initial scene.");
        } else {
            println!("Initial scene loaded successfully!");
        }

        if let GameSceneState::Normal(ref scene) = self.current_scene_state {
            println!("Scene loaded: {:?}", scene);
        }
        self.current_turn = GameTurn::Server;

        println!(
            "Session {} started with {} players.",
            self.id,
            self.party.len()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn load_scene_1_test() {
        //TODO
    }

    #[test]
    fn load_nonexistent_scene_test() {
        //TODO
    }
}
