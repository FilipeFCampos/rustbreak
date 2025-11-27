use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SceneOptions {
    pub a: String,
    pub b: String,
    pub c: String,
    pub d: String,
    pub id_correct: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GameScene {
    pub id: u8,
    pub description: String,
    pub code: String,
    pub options: SceneOptions,
    pub success_msg: String,
    pub error_msg: String,
}

#[derive(Debug, Clone)]
pub enum GameSceneState {
    Prelude,
    Normal(GameScene),
    Ending(GameScene),
}
