use crate::game::game_scene::{GameScene, GameSceneState};
use crate::game::player::Registry;
use std::fs::{File, OpenOptions};
use std::io::BufReader;
use uuid::Uuid;

pub enum GameTurn {
    Server,
    Player,
}

pub struct GameSession {
    pub id: Uuid,
    pub current_scene_state: GameSceneState,
    pub current_turn: GameTurn,
    pub party: Registry,
}

impl GameSession {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            current_turn: GameTurn::Server,
            current_scene_state: GameSceneState::Prelude,
            party: Registry::new(),
        }
    }

    pub fn load_scene(&mut self, path: &str) -> Result<(), &'static str> {
        let mut n_path = String::from("data/");
        n_path.push_str(path);

        match OpenOptions::new().read(true).open(n_path) {
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
