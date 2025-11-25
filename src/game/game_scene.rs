use serde::{Deserialize, Serialize};
// They all should have the same lifetime because they only exist mutually
// It's string for now because deserialization with from_reader requires this, not &'a str

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
