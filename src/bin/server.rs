use chrono::Local;
use rustbreak::common::{
    formatting::*,
    messages::{ChatMessage, MessageType},
    shared::*,
};
use rustbreak::game::player::{Player, Registry};
use std::{error::Error, sync::Arc};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
    sync::{Mutex, broadcast},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let listener = TcpListener::bind(format!("{ADDRESS}:{PORT}")).await?;
    let registry = Arc::new(Mutex::new(Registry::new()));

    welcome_message();

    let (sender, _) = broadcast::channel::<String>(128);

    loop {
        // Accept connection
        let (socket, addr) = listener.accept().await?;

        // Display connection
        println!(
            "┌─[{}] {GREEN}New Connection!{RESET}",
            Local::now().format("%H:%M:%S")
        );
        println!("└─ Address: {BLUE}{addr}{RESET}");

        // Clone the sender for this session
        let sender = sender.clone();
        // Set a receiver to listen for the sender
        let receiver = sender.subscribe();

        let registry_clone = Arc::clone(&registry);
        // Spawn a thread for handling each user's connection
        tokio::spawn(
            async move { handle_connection(socket, receiver, sender, registry_clone).await },
        );
    }
}

/// Prints the server's welcome message.
fn welcome_message() {
    println!("╔════════════════════════════════════════╗");
    println!("║                                        ║");
    println!(
        "║    {BLUE}SERVER RUNNING ON {CYAN}{}:{}{RESET}    ║",
        ADDRESS, PORT
    );
    println!("║    {YELLOW}Press Ctrl+C to shutdown{RESET}            ║");
    println!("║                                        ║");
    println!("╚════════════════════════════════════════╝");
}

/// Handles the connection and communication between a client and the server.
///
/// ### Parameters
/// - `socket`: The TCP socket server;
/// - `receiver`: A Receiver listening to the broadcast channel;
/// - `sender`: The broadcast Sender;
/// - `registry`: A Registry to store the current player _(client)_
async fn handle_connection(
    mut socket: TcpStream,
    mut receiver: broadcast::Receiver<String>,
    sender: broadcast::Sender<String>,
    registry: Arc<Mutex<Registry>>,
) {
    // TODO: Find a way to create a direct message channel between the server and each client.

    // Split socket into reader and writer
    let (reader, mut writer) = socket.split();
    let mut reader = BufReader::new(reader);
    let mut username = String::new();

    // Read the username of the user who just joined
    reader.read_line(&mut username).await.expect(&format!(
        "{RED}ERROR: Failed to read username from client.{RESET}"
    ));
    let username = username.trim().to_string();

    // Add new player to registry
    //// WORK IN PROGRESS
    let player = Player::new(username.clone());
    let mut registry_locked = registry.lock().await;
    match registry_locked.add(player) {
        None => {}
        Some(_) => {
            eprintln!("{RED}ERROR: Username [{}] already exists.{RESET}", username);
            // Send "this username is already taken" message
            // All this might change so take it with a grain of salt
            let error_msg = ChatMessage {
                username: username.clone(),
                content: format!(
                    "ERROR: Username [{}] already exists. Please pick another one.",
                    username
                ),
                timestamp: get_time(),
                message_type: MessageType::SystemNotification,
            };
            let error_msg_json = serde_json::to_string(&error_msg).unwrap();
            writer.write_all(error_msg_json.as_bytes()).await.unwrap();
            writer.write_all(b"\n").await.unwrap();
        }
    }
    drop(registry_locked);
    //// WORK IN PROGRESS ENDS HERE

    // Send join message
    let join_msg = ChatMessage {
        username: username.clone(),
        content: "joined the chat!".to_string(),
        timestamp: get_time(),
        message_type: MessageType::SystemNotification,
    };
    let join_msg_json = serde_json::to_string(&join_msg).unwrap();
    sender.send(join_msg_json).unwrap();

    let mut line = String::new();
    loop {
        tokio::select! {
            // Handle messages sent by the client
            result = reader.read_line(&mut line) => {
                if result.unwrap() == 0 {
                    break;
                }
                // Broadcast a user message
                let msg = ChatMessage {
                    username: username.clone(),
                    content: line.trim().to_string(),
                    timestamp: get_time(),
                    message_type: MessageType::UserMessage,
                };
                // This converts `ChatMessage` to a json object
                let msg_json = serde_json::to_string(&msg).unwrap();
                println!("┌─[{}] {YELLOW}{}{RESET}\n└─ Message: {BLUE}{}{RESET}", msg.timestamp, username, msg.content);
                // The json object is sent to the client
                sender.send(msg_json).unwrap();
                line.clear(); // Clear the buffer
            }
            // Handle incoming broadcasts and send them to the clients
            result = receiver.recv() => {
                let msg = result.unwrap();
                writer.write_all(msg.as_bytes()).await.unwrap();
                writer.write_all(b"\n").await.unwrap();
            }
        }
    }

    // Removes the current player from the registry
    let mut registry_locked = registry.lock().await;
    let _ = registry_locked.remove(username.clone());
    drop(registry_locked);

    // Send leave message
    let leave_msg = ChatMessage {
        username: username.clone(),
        content: "left the chat".to_string(),
        timestamp: get_time(),
        message_type: MessageType::SystemNotification,
    };

    println!(
        "├─[{}] {RED}{} disconnected.{RESET}",
        leave_msg.timestamp, username
    );

    let leave_msg_json = serde_json::to_string(&leave_msg).unwrap();
    sender.send(leave_msg_json).unwrap();
}
