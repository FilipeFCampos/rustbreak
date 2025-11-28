use chrono::Local;
use rustbreak::common::{
    formatting::*,
    messages::{ChatMessage, EventSignal, MessageType},
    shared::*,
};
use rustbreak::game::game_scene::GameSceneState;
use rustbreak::game::game_session::{GameEvent, GameSession};
use rustbreak::handlers::server_handlers::{
    ServerSessions, register_player, remove_player
};
use std::collections::HashMap;
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

    // ANTES: O servidor armazenava (GameSession, Sender)
    // Problema: GameSession era CLONADO quando enviado ao game_loop, então cada player atualizava uma CÓPIA diferente.
    // Resultado: game_loop nunca via as respostas dos jogadores -> ficava eternamente em "Aguardando mais respostas...".
    // AGORA: GameSession fica dentro de Arc<Mutex<_>>
    // significa que TODOS os handlers (server, players e game_loop) compartilham exatamente a MESMA sessão.
    //agora: answer -> session.update() -> game_loop vê atualização
    let sessions: ServerSessions = Arc::new(Mutex::new(HashMap::<
        Uuid,
        (Arc<Mutex<GameSession>>, Sender<String>),
    >::new()));

    welcome_message();

    loop {
        let (socket, addr) = listener.accept().await?;

        println!(
            "┌─[{}] {GREEN}New Connection!{RESET}",
            Local::now().format("%H:%M:%S")
        );
        println!("└─ Address: {BLUE}{addr}{RESET}");

        let sessions_clone = Arc::clone(&sessions);
        tokio::spawn(async move { handle_connection(socket, sessions_clone, addr).await });
    }
}

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

async fn handle_connection(
    mut socket: TcpStream,
    sessions: ServerSessions,
    addr: core::net::SocketAddr,
) {
    let (reader, mut writer) = socket.split();
    let mut reader = BufReader::new(reader);
    let mut username = String::new();
    let mut party_id: Option<Uuid> = None;
    let mut channel_receiver: Option<Receiver<String>> = None;

    // LOGIN + REGISTRO DO PLAYER
    loop {
        username.clear();

        match reader.read_line(&mut username).await {
            Ok(0) => return,
            Ok(_) => {}
            Err(e) => {
                eprintln!("{RED}ERROR reading from {addr}: {e}{RESET}");
                return;
            }
        }

        let trimmed = username.trim().to_string();
        if trimmed.is_empty() {
            continue;
        }

        match register_player(&sessions, trimmed.clone()).await {
            Ok((id, receiver)) => {
                party_id = Some(id);
                channel_receiver = Some(receiver);
            }
            Err(err) => {
                let error_signal = EventSignal::Error(err.clone());
                let json = serde_json::to_string(&error_signal).unwrap();

                if writer.write_all(json.as_bytes()).await.is_err()
                    || writer.write_all(b"\n").await.is_err()
                {
                    return;
                }
                continue;
            }
        }

        username = trimmed;

        let ok_signal = EventSignal::Ok(username.clone());
        let json = serde_json::to_string(&ok_signal).unwrap();

        if writer.write_all(json.as_bytes()).await.is_err()
            || writer.write_all(b"\n").await.is_err()
        {
            remove_player(&sessions, &username, None).await;
            return;
        }

        break;
    }

    let join_msg = ChatMessage {
        username: username.clone(),
        content: "joined the chat!".into(),
        timestamp: get_time(),
        message_type: MessageType::SystemNotification,
    };

    let join_json = serde_json::to_string(&join_msg).unwrap();
    broadcast_message(join_json, party_id.unwrap(), &sessions).await;

    println!(
        "├─[{}] {GREEN}'{}' joined the chat!{RESET}",
        join_msg.timestamp, username
    );

    // INICIAR GAME LOOP QUANDO TIVER 3 PLAYERS
    {
        let guard = sessions.lock().await;

        if let Some((session_arc, sender)) = guard.get(&party_id.unwrap()) {
            let s = session_arc.lock().await;
            // ANTES: isso rodava com COPIA -> game_loop nunca recebia update
            // AGORA: usa Arc<Mutex<>> -> todos compartilham a MESMA session
            if s.party.len() == 3 {
                let session_shared = Arc::clone(session_arc);
                let sender_clone = sender.clone();

                tokio::spawn(async move {
                    game_loop(session_shared, sender_clone).await;
                });
            }
        }
    }
    let mut line = String::new();
    let mut receiver = channel_receiver.take().unwrap();

    loop {
        tokio::select! {
            // CLIENT -> SERVIDOR
            result = reader.read_line(&mut line) => {
                match result {
                    Ok(0) => break,
                    Ok(_) => {
                        let content = line.trim().to_string();
                        if content.starts_with("/answer ") {
                            let answer = content["/answer ".len()..].trim().to_string();
                            let party_id = party_id.unwrap();

                            // CRITICAL BLOCK: agora session.update() altera A VERDADEIRA sessions compartilhada
                            // ANTES: session era clonado -> update() acontecia numa cópia descartada -> game_loop nunca via nada.
                            // AGORA: session_arc.lock() -> atualiza MESMO objeto.
                            let (sender_clone, json_to_send) = {
                                let mut guard = sessions.lock().await;

                                let (session_arc, sender) = guard.get_mut(&party_id).unwrap();
                                let mut session = session_arc.lock().await;

                                let feedback_opt = session.update(GameEvent::PlayerAnswer {
                                    username: username.clone(),
                                    answer,
                                });

                                if feedback_opt.is_none() {
                                    println!(
                                        "┌─[{}] Aguardando mais respostas para a party {}...",
                                        get_time(),
                                        party_id
                                    );
                                    line.clear();
                                    continue;
                                }

                                let feedback = feedback_opt.unwrap();

                                let msg = ChatMessage {
                                    username: "System".into(),
                                    content: feedback,
                                    timestamp: get_time(),
                                    message_type: MessageType::SystemNotification,
                                };

                                let json = serde_json::to_string(&msg).unwrap();

                                (sender.clone(), json)
                            };
                            // ***** END CRITICAL BLOCK *****

                            let _ = sender_clone.send(format!("{}\n", json_to_send));
                            line.clear();
                            continue;
                        }

                        // CHAT NORMAL
                        let msg = ChatMessage {
                            username: username.clone(),
                            content: content.clone(),
                            timestamp: get_time(),
                            message_type: MessageType::UserMessage,
                        };

                        let json = serde_json::to_string(&msg).unwrap();
                        broadcast_message(json, party_id.unwrap(), &sessions).await;

                        line.clear();
                    }
                    Err(_) => break
                }
            }
            // SERVIDOR -> CLIENT
            result = receiver.recv() => {
                match result {
                    Ok(msg) => {
                        writer.write_all(msg.as_bytes()).await.unwrap();
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        }
    }
    remove_player(&sessions, &username, party_id).await;

    let leave_msg = ChatMessage {
        username: username.clone(),
        content: "left the chat".into(),
        timestamp: get_time(),
        message_type: MessageType::SystemNotification,
    };

    let leave_json = serde_json::to_string(&leave_msg).unwrap();
    broadcast_message(leave_json, party_id.unwrap(), &sessions).await;

    println!(
        "├─[{}] {YELLOW}'{}' disconnected.{RESET}",
        leave_msg.timestamp, username
    );
}

// BROADCAST
async fn broadcast_message(msg: String, party_id: Uuid, sessions: &ServerSessions) {
    let guard = sessions.lock().await;

    if let Some((_, sender)) = guard.get(&party_id) {
        let _ = sender.send(format!("{}\n", msg));
    }
}

// GAME LOOP — agora funciona corretamente
// ANTES: recebia um GameSession COPIADO, ficava congelado esperando respostas que nunca chegavam.
// AGORA: recebe Arc<Mutex<GameSession>> qualquer update via /answer é visível instantaneamente
async fn game_loop(session: Arc<Mutex<GameSession>>, sender: Sender<String>) {
    {
        let mut s = session.lock().await;
        s.begin_game();
    }

    let start_msg = ChatMessage {
        username: "System".into(),
        content: "Game started! The adventure begins...".into(),
        timestamp: get_time(),
        message_type: MessageType::SystemNotification,
    };

    let json = serde_json::to_string(&start_msg).unwrap();
    let _ = sender.send(format!("{}\n", json));

    {
        let s = session.lock().await;
        if let GameSceneState::Normal(scene) = &s.current_scene_state {
            let scene_signal = EventSignal::Scene(scene.clone());
            let scene_json = serde_json::to_string(&scene_signal).unwrap();
            let _ = sender.send(format!("{}\n", scene_json));
        }
    }
}
