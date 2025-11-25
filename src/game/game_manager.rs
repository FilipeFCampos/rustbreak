//! Implementation of the game manager.

use crate::game::game_scene::GameScene;
use std::{fs::OpenOptions, io::BufReader};

pub struct GameManager {
    current_scene: GameScene,
}

impl GameManager {
    pub fn load_scene(&mut self, path: &str) -> Result<(), &'static str> {
        let mut n_path = String::from("data/");
        n_path.push_str(path);

        match OpenOptions::new().read(true).open(n_path) {
            Ok(file) => {
                let reader = BufReader::new(file);

                match serde_json::from_reader(reader) {
                    Ok(game_scene) => {
                        self.current_scene = game_scene;
                        Ok(())
                    }
                    Err(_) => Err("Failed to deserialize game scene."),
                }
            }
            Err(_) => Err("Failed to load scene."),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn load_scene_1_test()
    {
        //TODO

    }

    #[test]
    fn load_nonexistent_scene_test() {
        //TODO
    }
}
