use crate::game::game_scene::{GameScene, GameSceneState};
use crate::game::player::Player;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use uuid::Uuid;

pub const MAX_PLAYERS_PER_SESSION: usize = 3;

#[derive(Clone)]
pub enum GameTurn {
    Server,
    Player,
}

pub enum GameEvent {
    PlayerAnswer { username: String, answer: String },
}

#[derive(Clone)]
pub struct GameSession {
    pub id: Uuid,
    pub current_scene_state: GameSceneState,
    pub current_turn: GameTurn,
    pub party: HashSet<Player>,
    // NOVO: agora rastreamos respostas dos jogadores
    // String -> username, bool -> acertou(true) / errou(false)
    // Isso permite aplicar a regra “se 2 acertarem, sucesso; se 2 errarem, erro”
    pub answers: HashMap<String, bool>,
}

impl GameSession {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            current_turn: GameTurn::Server,
            current_scene_state: GameSceneState::Prelude,
            party: HashSet::new(),
            answers: HashMap::new(),
        }
    }

    pub fn add_player(&mut self, username: String) -> Result<(), String> {
        if self.contains(&username) {
            return Err(format!("Player already registered: {}", username));
        } else if self.party.len() >= MAX_PLAYERS_PER_SESSION {
            return Err(format!("Party already full: {}", username));
        }

        if self.party.insert(Player::new(username.clone())) {
            Ok(())
        } else {
            Err("Cannot add player twice".to_string())
        }
    }

    pub fn contains(&self, username: &String) -> bool {
        self.party.iter().any(|p| p.username == *username)
    }

    pub fn remove_player(&mut self, username: &String) {
        self.party.retain(|p| p.username != *username);
    }

    /// NOVO: lógica de votação por maioria (>= 2 acertos = sucesso)
    /// Antes: retornava mensagem por jogador individual
    /// Agora: só retorna mensagem quando TODOS responderam
    pub fn update(&mut self, event: GameEvent) -> Option<String> {
        match event {
            GameEvent::PlayerAnswer { username, answer } => {
                let scene = match &self.current_scene_state {
                    GameSceneState::Normal(scene) => scene,
                    _ => return None,
                };

                // jogador já respondeu
                if self.answers.contains_key(&username) {
                    return None;
                }

                // verifica acerto individual
                let correct =
                    answer.trim().eq_ignore_ascii_case(&scene.options.id_correct);

                // registra resposta
                self.answers.insert(username.clone(), correct);

                // se ainda falta jogador responder, nada acontece
                if self.answers.len() < self.party.len() {
                    return None;
                }

                // TODOS responderam -> aplicar maioria
                let correct_count = self.answers.values().filter(|v| **v).count();
                let wrong_count = self.party.len() - correct_count;

                // limpa respostas para próxima rodada
                self.answers.clear();
                 // regra principal: 2 ou mais acertos = sucesso
                if correct_count >= 2 {
                    Some(format!(
                        "{} players got it right! {}",
                        correct_count, scene.success_msg
                    ))
                } else {
                    Some(format!(
                        "{} players got it wrong! {}",
                        wrong_count, scene.error_msg
                    ))
                }
            }
        }
    }

    pub fn get_scene_json(&self) -> Option<String> {
        match &self.current_scene_state {
            GameSceneState::Normal(scene) => serde_json::to_string(scene).ok(),
            _ => None,
        }
    }

    fn load_scene(&mut self, path: &str) -> Result<(), &'static str> {
        let mut full_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        full_path.push("data");
        full_path.push(path);

        let file = File::open(&full_path).map_err(|_| "Failed to load scene.")?;
        let reader = BufReader::new(file);

        let game_scene: GameScene = serde_json::from_reader(reader).map_err(|_| "Failed to deserialize game scene.")?;
        self.current_scene_state = GameSceneState::Normal(game_scene);
        Ok(())
    }

    pub fn toggle_turn(&mut self) {
        self.current_turn = match self.current_turn {
            GameTurn::Server => GameTurn::Player,
            GameTurn::Player => GameTurn::Server,
        };
    }

    pub fn begin_game(&mut self) {
        self.current_scene_state = GameSceneState::Prelude;
        if let Err(_) = self.load_scene("scene_1.json") {
            println!("Error loading initial scene.");
        } else {
            println!("Initial scene loaded successfully!");
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
