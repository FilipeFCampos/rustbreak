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
    PlayerJoined(String),
    PlayerAnswer { username: String, answer: String },
}

pub enum UpdateResult {
    Advance(String),
    Continue,
    EndGame,
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
    pub remaining_answers: i8,
    remaining_scenes: Vec<String>,
}

impl GameSession {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            current_turn: GameTurn::Server,
            current_scene_state: GameSceneState::Prelude,
            party: HashSet::new(),
            answers: HashMap::new(),
            remaining_answers: 3,
            remaining_scenes: vec![
                "scene_1".into(),
                "scene_2".into(),
                "scene_3".into(),
                "scene_4".into(),
                "scene_5".into(),
                "scene_6".into(),
                "scene_7".into(),
            ],
        }
    }

    pub fn add_player(&mut self, username: &String) -> Result<(), String> {
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
    pub fn update(&mut self, event: GameEvent) -> UpdateResult {
        // TODO: controlar isso aqui pra só permitir alterar o estado quando o jogo realmente tiver começado
        match event {
            GameEvent::PlayerAnswer { username, answer } => {
                let scene = match &self.current_scene_state {
                    GameSceneState::Normal(scene) => scene,
                    _ => return UpdateResult::Continue,
                };

                // verifica acerto individual
                let correct = answer
                    .trim()
                    .eq_ignore_ascii_case(&scene.options.id_correct);

                // registra resposta
                self.answers.insert(username.clone(), correct);

                // se ainda falta jogador responder, nada acontece
                if self.answers.len() < self.party.len() {
                    return UpdateResult::Continue;
                }

                // TODOS responderam -> aplicar maioria
                let correct_count = self.answers.values().filter(|v| **v).count();
                let wrong_count = self.party.len() - correct_count;

                // limpa respostas para próxima rodada
                self.answers.clear();
                let text_result: String;

                // regra principal: acertos > erros = sucesso
                if correct_count >= wrong_count {
                    text_result = scene.success_msg.clone();
                } else {
                    text_result = scene.error_msg.clone();
                    self.remaining_answers -= 1;
                }

                // Se acabou a quantidade de tentativas então acaba o jogo!
                if self.remaining_answers <= 0 {
                    return UpdateResult::EndGame;
                }

                let mut count_result = format!(
                    "{} jogadores acertaram e {} erraram! Vocês ainda têm {} tentativa(s)! \n",
                    correct_count, wrong_count, self.remaining_answers
                );

                count_result.push_str(&text_result);
                UpdateResult::Advance(count_result)
            }
            GameEvent::PlayerJoined(_) => UpdateResult::Continue,
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

        let game_scene: GameScene =
            serde_json::from_reader(reader).map_err(|_| "Failed to deserialize game scene.")?;
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
    #[test]
    fn load_scene_1_test() {
        //TODO
    }

    #[test]
    fn load_nonexistent_scene_test() {
        //TODO
    }
}
