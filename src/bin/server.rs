use chrono::Local;
use rustbreak::common::{
    formatting::*,
    messages::{ChatMessage, EventSignal, MessageType},
    shared::*,
};
use rustbreak::game::game_session::GameSession;
use rustbreak::game::game_scene::GameSceneState;
use rustbreak::handlers::server_handlers::{ServerSessions, register_player, remove_player};
use std::{error::Error, sync::Arc};
use tokio::sync::broadcast::{Receiver, Sender};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
    sync::{Mutex, broadcast},
};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let listener = TcpListener::bind(format!("{ADDRESS}:{PORT}")).await?;
    let sessions = Arc::new(Mutex::new(vec![(
        GameSession::new(),
        broadcast::channel::<String>(128).0,
    )]));

    welcome_message();

    loop {
        // Accept connection
        let (socket, addr) = listener.accept().await?;

        // Display connection
        println!(
            "┌─[{}] {GREEN}New Connection!{RESET}",
            Local::now().format("%H:%M:%S")
        );
        println!("└─ Address: {BLUE}{addr}{RESET}");

        let sessions_clone = Arc::clone(&sessions);
        // Spawn a thread for handling each user's connection
        tokio::spawn(async move { handle_connection(socket, sessions_clone, addr).await });
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
    sessions: Arc<Mutex<Vec<(GameSession, Sender<String>)>>>,
    addr: core::net::SocketAddr,
) {
    // Split socket into reader and writer
    let (reader, mut writer) = socket.split();
    let mut reader = BufReader::new(reader);
    let mut username = String::new();
    let mut party_id: Option<Uuid> = None;
    let mut channel_receiver: Option<Receiver<String>> = None;

    // Read the username of the user who just joined
    loop {
        username.clear();
        match reader.read_line(&mut username).await {
            Ok(0) => return,
            Ok(_) => {}
            Err(e) => {
                eprintln!("{RED}ERROR: Failed to read from {}: {}{RESET}", addr, e);
                return;
            }
        }

        let trimmed_username = username.trim().to_string();

        if trimmed_username.is_empty() {
            continue; // Ignore empty inputs and wait for the next one.
        }

        // Add new player to registry
        if let Ok((id, receiver)) = register_player(&sessions, trimmed_username.clone()).await {
            party_id = Some(id);
            channel_receiver = Some(receiver);
        } else {
            // Send "this username is already taken" message
            let error_msg = format!(
                "Username [{}] is already in use. Please try another.",
                trimmed_username
            );
            eprintln!(
                "┌─[{}] {BLUE}{}{RESET}\n└─ {RED}Denied: {}{RESET}",
                get_time(),
                addr,
                error_msg
            );

            let error_signal = EventSignal::Error(error_msg);
            let error_signal_json = serde_json::to_string(&error_signal).unwrap();

            if writer
                .write_all(error_signal_json.as_bytes())
                .await
                .is_err()
                || writer.write_all(b"\n").await.is_err()
            {
                return;
            }
            continue;
        }

        username = trimmed_username;

        let ok_signal = EventSignal::Ok(username.clone());
        let ok_signal_json = serde_json::to_string(&ok_signal).unwrap();

        if writer.write_all(ok_signal_json.as_bytes()).await.is_err()
            || writer.write_all(b"\n").await.is_err()
        {
            remove_player(&sessions, &username, None).await;
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
        broadcast_message(join_msg_json, party_id.unwrap(), &sessions).await;

        if let Some(id) = party_id {
            let mut sessions_guard = sessions.lock().await;

            if let Some((session, sender)) =
                sessions_guard.iter_mut().find(|(s, _)| s.id == id)
            {
                if session.party.len() == 3 {
                    session.begin_game();

                    let start_msg = ChatMessage {
                        username: "System".into(),
                        content: "Game started! The adventure begins...".into(),
                        timestamp: get_time(),
                        message_type: MessageType::SystemNotification,
                    };

                    let json = serde_json::to_string(&start_msg).unwrap();
                    let _ = sender.send(json);

                    if let GameSceneState::Normal(scene) = &session.current_scene_state {
                        let scene_signal = EventSignal::Scene(scene.clone());
                        let scene_json = serde_json::to_string(&scene_signal).unwrap();
                        let _ = sender.send(scene_json);
                    }
                }
            }
        }
        break;
    }

    // Chat loop
    let mut line = String::new();
    let mut receiver = channel_receiver.take().expect("Receiver should exist here");

    loop {
        tokio::select! {
            // Handle messages sent by the client
            result = reader.read_line(&mut line) => {
                match result {
                    Ok(0) => break,
                    Ok(_) => {
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
                        broadcast_message(msg_json, party_id.unwrap(), &sessions).await;
                        line.clear(); // Clear the buffer
                    },
                    Err(_) => {}
                }
            }
            // Handle incoming broadcasts and send them to the clients
            result = receiver.recv() => {
                match result {
                    Ok(msg) => {
                        writer.write_all(msg.as_bytes()).await.unwrap();
                        writer.write_all(b"\n").await.unwrap();
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => {
                        continue;
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        break;
                    }
                }
            }
        }
    }

    // Disconnect
    remove_player(&sessions, &username, party_id).await;

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
    broadcast_message(leave_msg_json, party_id.unwrap(), &sessions).await;
}

async fn broadcast_message(msg: String, party_id: Uuid, sessions: &ServerSessions) {
    let sessions = sessions.lock().await;
    let party = sessions.iter().find(|(s, _)| s.id == party_id);

    if let Some((party, sender)) = party {
        let _ = sender.send(msg);
    }

    drop(sessions);
}

// TODO: Move 'add new player to registry' code here
fn new_player_handler() {
    todo!()
}
