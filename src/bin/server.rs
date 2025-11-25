use chrono::Local;
use rustbreak::common::{
    formatting::*,
    messages::{ChatMessage, EventSignal, MessageType},
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
        tokio::spawn(async move {
            handle_connection(socket, receiver, sender, registry_clone, addr).await
        });
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
    addr: core::net::SocketAddr,
) {
    // Split socket into reader and writer
    let (reader, mut writer) = socket.split();
    let mut reader = BufReader::new(reader);
    let mut username = String::new();

    // Read the username of the user who just joined
    loop {
        username.clear();
        match reader.read_line(&mut username).await {
            Ok(0) => return,
            Ok(_) => {},
            Err(e) => { eprintln!("{RED}ERROR: Failed to read from {}: {}{RESET}", addr, e);
                return;
            }
        }

        let trimmed_username = username.trim().to_string();

        if trimmed_username.is_empty() {
            continue; // Ignore empty inputs and wait for the next one.
        }
        
        let mut registry_locked = registry.lock().await;

        // Add new player to registry
        if registry_locked.add(Player::new(trimmed_username.clone())).is_some() {
            // Send "this username is already taken" message
            let error_msg = format!("Username [{}] is already in use. Please try another.", trimmed_username);
            eprintln!("┌─[{}] {BLUE}{}{RESET}\n└─ {RED}Denied: {}{RESET}", get_time(), addr, error_msg);

            let error_signal = EventSignal::Error(error_msg);
            let error_signal_json = serde_json::to_string(&error_signal).unwrap();
            
            if writer.write_all(error_signal_json.as_bytes()).await.is_err() || writer.write_all(b"\n").await.is_err() {
                return; 
            }
        
            drop(registry_locked);
            continue; 
        }

        drop(registry_locked);
        username = trimmed_username;

        let ok_signal = EventSignal::Ok(username.clone());
        let ok_signal_json = serde_json::to_string(&ok_signal).unwrap();

        if writer.write_all(ok_signal_json.as_bytes()).await.is_err() || writer.write_all(b"\n").await.is_err() {
             let mut registry_locked = registry.lock().await;
             registry_locked.remove(username.clone());
             return;
        }

        // Send join message
        let join_msg = ChatMessage {
            username: username.clone(),
            content: "joined the chat!".to_string(),
            timestamp: get_time(),
            message_type: MessageType::SystemNotification,
        };
        println!(
            "├─[{}] {GREEN}'{}' joined the chat!{RESET}",
            join_msg.timestamp, username
        );
        let join_msg_json = serde_json::to_string(&join_msg).unwrap();
        sender.send(join_msg_json).unwrap();

        break;
    }

    //Chat loop
    let mut line = String::new();

    // DANGER: Using `unwrap()` on network I/O operations is risky.
    // If the connection drops unexpectedly (e.g. "Connection reset by peer") or the
    // channel fails, `unwrap()` will panic. This crashes the client task immediately
    // and might prevent proper cleanup (like removing the user from the registry).
    // TODO: Replace `unwrap()` with `match` or `if let` for graceful error handling.
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

    // Disconnect
    let mut registry_locked = registry.lock().await;
    registry_locked.remove(username.clone());
    drop(registry_locked);

    // Send leave message
    let leave_msg = ChatMessage {
        username: username.clone(),
        content: "left the chat".to_string(),
        timestamp: get_time(),
        message_type: MessageType::SystemNotification,
    };

    println!(
        "├─[{}] {YELLOW}'{}' disconnected.{RESET}",
        leave_msg.timestamp, username
    );

    let leave_msg_json = serde_json::to_string(&leave_msg).unwrap();
    sender.send(leave_msg_json).unwrap();
}

// TODO: Move 'add new player to registry' code here
fn new_player_handler() {
    todo!()
}