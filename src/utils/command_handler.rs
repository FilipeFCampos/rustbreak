use uuid::Uuid;

/// Models common command errors to be returned by `ClientMessage.eval()`.
/// 
/// ### Values
/// - `InvalidCommand`: The command is invalid.
/// - `FaultyPrefix` The prefix is invalid. I.e the message does not start with `/`.
pub enum CommandError {
    // 
    InvalidCommand,
    FaultyPrefix,
}

/// Represents the different types of actions a player can perform.
/// 
/// Stores the Uuid of the player who requested the action.
pub enum Action {
    Help(Uuid),
    Move(Uuid),
    Attack(Uuid),
    Ready(Uuid),
    Exit(Uuid),
}

/// Is a message sent by a player to the telnet server.
/// 
/// Stores the Uuid of that player and the message as a String.
pub struct ClientMessage {
    pub client_id: Uuid,
    pub message: String,
}

fn split_message(msg: &String) -> Option<(char, &str)> {
    match msg.chars().nth(0) {
        Some(prefix) => Some((prefix, &msg[1..])),
        None => None,
    }
}

impl ClientMessage {
    /// Checks whether a message is valid or not.
    /// 
    /// ### Returns
    /// - `Action` if the typed message is a valid command;
    /// - `CommandError` if the message to be evaluated is invalid.
    pub fn eval(&self) -> Result<Action, CommandError> {
        match split_message(&self.message) {
            Some(('/', msg)) => match msg {
                "help" => Ok(Action::Help(self.client_id)),
                "move" => Ok(Action::Move(self.client_id)),
                "attack" => Ok(Action::Attack(self.client_id)),
                "ready" => Ok(Action::Ready(self.client_id)),
                "exit" => Ok(Action::Ready(self.client_id)),
                _ => Err(CommandError::InvalidCommand),
            },
            Some((_, _)) => Err(CommandError::FaultyPrefix),
            None => Err(CommandError::InvalidCommand),
        }
    }
}
