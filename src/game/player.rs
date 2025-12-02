use uuid::Uuid;

/// Models a player connected to the server.
#[derive(Clone, Hash, Eq, PartialEq)]
pub struct Player {
    pub id: Uuid,
    pub username: String,
}

impl Player {
    pub fn new(username: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            username,
        }
    }
}
