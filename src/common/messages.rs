use serde::{Deserialize, Serialize};

/// Represents a message sent to the chat.
///
/// ### Fields
/// - `username`: The name of the user who sent the message;
/// - `content`: The message content itself;
/// - `timestamp`: The exact date and time the message was sent;
/// - `message type`: The type of the message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub username: String,
    pub content: String,
    pub timestamp: String,
    pub message_type: MessageType,
}

/// Models the different types of message that can be sent using the chat.
///
/// ### Types
/// - `UserMessage`: An ordinary message sent by a user;
/// - `SystemNotification`: A notification sent from the server to a user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageType {
    UserMessage,
    SystemNotification,
    // Signal(EventSignal),
}

// TODO: Implement enum (EventSignal) to serve as a way of sending signals
// from the server to a specific client e.g. signal to 'kick' that player
// or cause 'damage'.
