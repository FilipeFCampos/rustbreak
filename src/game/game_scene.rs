use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
struct SceneOptions {
    a: String,
    b: String,
    c: String,
    d: String,
    id_correct: char,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GameScene {
    id: u8,
    description: String,
    code: String,
    options: SceneOptions,
    success_msg: String,
    error_msg: String,
}

pub enum GameSceneState {
    Prelude,
    Normal(GameScene),
    Ending(GameScene),
}
