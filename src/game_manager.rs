use nanoid::nanoid;

use crate::utils::command_handler::Action;

struct GameManager {
    match_id: String,
    n_players: u32,
}

impl GameManager {
    fn new() -> Self {
        Self {
            match_id: nanoid!(5),
            n_players: 0,
        }
    }

    fn act(action: Action) {
        match action {
            Action::Help => println!("AAAAAAAAAAAAAAAAA"),
            _ => {}
        }
    }
}
