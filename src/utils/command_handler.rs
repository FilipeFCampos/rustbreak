use uuid::Uuid;

/// Models common command errors to be returned by `ClientMessage.eval()`.
///
/// ### Values
/// - `InvalidCommand`: The command is invalid;
/// - `MissingArguments`: The command has missing required arguments.
/// ```
pub enum CommandError {
    InvalidCommand,
    MissingArguments,
}

/// Represents the different types of actions a player can perform.
///
/// Stores the Uuid of the player who requested the action.
pub enum Action {
    Help,
    Ready,
    Exit,
    Talk(String),
    Move(String),
    Attack(String),
    Connect(String),
}

/// Is a message sent by a player to the telnet server.
///
/// Stores the Uuid of that player and the message as a String.
pub struct ClientMessage {
    pub client_id: Uuid,
    pub content: String,
}

impl ClientMessage {
    /// Checks whether a message is a valid command or not.
    ///
    /// ### Returns
    /// - `Ok(Action)` if the typed message is a valid command;
    /// - `Err(CommandError)` if a `/` prefixed message to be evaluated is a invalid command;
    /// - `Err(MissingArguments)` if a command has missing arguments.
    pub fn eval(&self) -> Result<Action, CommandError> {
        let message = self.content.trim();

        let (command, args) = if let Some((cmd, arg)) = message.split_once(' ') {
            match cmd.split_once('/') {
                Some((_, command)) => (command, Some(arg)),
                _ => return Ok(Action::Talk(self.content.clone())),
            }
        } else {
            match message.split_once('/') {
                Some((_, command)) => (command, None),
                _ => return Ok(Action::Talk(self.content.clone())),
            }
        };

        match command {
            "move" => args.map(|a| Action::Move(a.to_string())),
            "attack" => args.map(|a| Action::Attack(a.to_string())),
            "connect" => args.map(|a| Action::Connect(a.to_string())),
            "ready" => Some(Action::Ready),
            "exit" => Some(Action::Exit),
            "help" => Some(Action::Help),
            _ => return Err(CommandError::InvalidCommand),
        }
        .ok_or(CommandError::MissingArguments)
    }
}
