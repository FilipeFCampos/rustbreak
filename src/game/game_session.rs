use crate::game::game_scene::{GameScene, GameSceneState};
use crate::game::player::Player;
use std::collections::{HashMap, HashSet, VecDeque};
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use std::thread::sleep;
use std::time::Duration;
use uuid::Uuid;

pub const MAX_PLAYERS_PER_SESSION: usize = 3;

pub enum GameEvent {
    PlayerJoined(String),
    PlayerAnswer { username: String, answer: String },
    AdvanceTurn,
}

pub enum UpdateResult {
    Advance(String),          // String guarda a mensagem de erro ou de acerto do cenário!
    Continue(Option<String>), // tem String pra exibir a resposta do usuário diante da questão, None pra qualquer outra coisa
    GameOver(String),         // guarda a mensagem de erro do cenário
}

#[derive(Clone)]
pub struct GameSession {
    pub id: Uuid,
    pub current_scene_state: GameSceneState,
    pub party: HashSet<Player>,
    pub has_started: bool,
    answers: HashMap<String, bool>,
    remaining_answers: i8,
    remaining_scenes: VecDeque<String>,
}

impl GameSession {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            current_scene_state: GameSceneState::Prelude,
            party: HashSet::new(),
            answers: HashMap::new(),
            has_started: false,
            remaining_answers: 3,
            remaining_scenes: vec![
                "scene_1".into(),
                "scene_2".into(),
                "scene_3".into(),
                "scene_4".into(),
                "scene_5".into(),
                "scene_6".into(),
                "scene_7".into(),
            ]
            .into(),
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

    pub fn update(&mut self, event: GameEvent) -> UpdateResult {
        // Não permite atualizar nada do estado do jogo se não tiver iniciado ainda.
        if !self.has_started {
            return UpdateResult::Continue(None);
        }

        match event {
            GameEvent::PlayerAnswer { username, answer } => {
                let scene = match &self.current_scene_state {
                    GameSceneState::Normal(scene) => scene,
                    _ => return UpdateResult::Continue(None),
                };

                // verifica acerto individual
                let correct = answer
                    .trim()
                    .eq_ignore_ascii_case(&scene.options.id_correct);

                // registra resposta
                self.answers.insert(username.clone(), correct);

                // se ainda falta jogador responder, nada acontece
                if self.answers.len() < self.party.len() {
                    return UpdateResult::Continue(Some(answer));
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
                    return UpdateResult::GameOver(scene.error_msg.clone());
                }

                let mut count_result = format!(
                    "{} jogadores acertaram e {} erraram! Vocês ainda têm {} tentativa(s)! \n",
                    correct_count, wrong_count, self.remaining_answers
                );

                count_result.push_str(&text_result);
                UpdateResult::Advance(count_result)
            }
            GameEvent::PlayerJoined(_) => UpdateResult::Continue(None),
            _ => UpdateResult::Continue(None),
        }
    }

    pub fn next_scene(&mut self) {
        match self.remaining_scenes.pop_front() {
            None => {}
            Some(scene) => {
                let _ = self.load_scene(scene.as_str());
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
        full_path.push(format!("{}.json", path));

        let file = File::open(&full_path).map_err(|_| "Failed to load scene.")?;
        let reader = BufReader::new(file);

        let game_scene: GameScene =
            serde_json::from_reader(reader).map_err(|_| "Failed to deserialize game scene.")?;
        self.current_scene_state = GameSceneState::Normal(game_scene);
        Ok(())
    }

    pub fn get_prelude_text() -> String {
        r#"
Iniciando conexão com quantum.imd.ufrn.br...
Protocolo TELNET handshake... OK.
...Conexão segura estabelecida.

BEM-VINDOS, Investigadores.

(A tela pisca brevemente...)
(A luz do terminal falha...)

[PANIC: unexpected kernel trap]
[SEGFAULT @0x00ffd19a]
[FATAL: recursion detected in non-recursive function]
[STACK OVERFLOW PROTECTOR: DISARMED]

(A tela estabiliza novamente.)

Data Estelar: [Data Atual]  
Status do Sistema: **CRÍTICO**  
Local: Instituto Metrópole Digital (IMD), UFRN.

O orgulho do IMD, o supercomputador 'Potiguara-Q', foi ativado esta manhã. Escrito inteiramente em Rust para garantir segurança e performance quântica, ele era a promessa de uma nova era…

Porém…

A promessa falhou.

Não do jeito que vocês devem estar pensando. O 'Potiguara-Q' não 'crashou'. Ele... 'compilou'. A realidade do campus da UFRN foi tratada como seu código-fonte, e ele encontrou 'bugs'.

Agora, o computador está ativamente tentando 'corrigir' a realidade, causando anomalias catastróficas. O sistema está em caos.

Uma sub-rotina de segurança de baixo nível, o 'Crab Guardian', conseguiu contatar vocês. Ele identificou seus terminais como pertencentes a usuários que entendem a Lógica por trás da Linguagem.

Sem mais enrolação, vamos para vossa missão:

Entrar no sistema.
Encontrar as anomalias.
E forçar um CONSENSO.

Vocês devem 'corrigir o código' da realidade, juntos.

Mas cuidado.
O 'Crab Guardian' detectou... 'interferência'. As anomalias não parecem totalmente acidentais…

Enfim... O conhecimento, a discussão e o consenso são suas únicas armas.

[STATUS DA SESSÃO: JOGADORES CONECTADOS: 3]

[HOST]: O chat de vocês está aberto. Discutam.  
O Saguão espera.

[HOST]: Boa sorte. Vocês vão precisar.
"#
        .into()
    }

    pub fn begin_game(&mut self) {
        if self.has_started {
            return;
        }

        self.has_started = true;
        if let Some(scene) = self.remaining_scenes.pop_front() {
            if self.load_scene(scene.as_str()).is_ok() {
                println!("Initial scene loaded successfully!");
            } else {
                println!("Error loading initial scene.");
            }
        }

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
