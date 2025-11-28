use chrono::Local;
use rustbreak::common::{
    formatting::*,
    messages::{ChatMessage, EventSignal, MessageType},
    shared::*,
};
use rustbreak::game::game_scene::GameSceneState;
use rustbreak::game::game_session::{
    GameEvent, GameSession, UpdateResult, MAX_PLAYERS_PER_SESSION,
};
use rustbreak::handlers::server_handlers::{ServerSessions, SessionEntry};
use std::collections::HashMap;
use std::{error::Error, sync::Arc};
use tokio::sync::broadcast::Sender;
use tokio::sync::mpsc;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
    sync::{broadcast, Mutex},
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
    let sessions: ServerSessions = Arc::new(Mutex::new(HashMap::<Uuid, SessionEntry>::new()));

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
    let mut broadcast: Option<broadcast::Receiver<String>> = None;
    let mut event_channel: Option<mpsc::Sender<GameEvent>> = None;
    let mut event_receiver: Option<mpsc::Receiver<GameEvent>> = None;

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

        // fazendo aqui o shadowing
        let username = username.trim().to_string();
        if username.is_empty() {
            continue;
        }

        // escopo diferente pra tentar adicionar user!
        {
            let mut sessions = sessions.lock().await;
            let mut available_id: Option<Uuid> = None;

            // encontra alguma partida com vagas!
            for (id, entry) in sessions.iter() {
                let session = entry.session.lock().await;
                if session.party.len() < MAX_PLAYERS_PER_SESSION {
                    available_id = Some(*id);
                    break;
                }
            }

            let res: Result<(Uuid, broadcast::Receiver<String>, mpsc::Sender<GameEvent>), String> =
                match available_id {
                    // não tem party !!
                    None => {
                        let (broadcast_sender, _) = broadcast::channel::<String>(128);
                        let (event_sender, event_receiver) = mpsc::channel::<GameEvent>(128);

                        let new_session = Arc::new(Mutex::new(GameSession::new()));
                        let party_id = new_session.lock().await.id;

                        sessions.insert(
                            party_id,
                            SessionEntry::new(
                                new_session.clone(),
                                broadcast_sender.clone(),
                                event_sender.clone(),
                            ),
                        );

                        let entry = sessions.get_mut(&party_id).unwrap();

                        // adiciona jogador numa session agora protegida por Mutex
                        let mut session_lock = entry.session.lock().await;
                        match session_lock.add_player(&username) {
                            Ok(_) => {
                                let receiver = broadcast_sender.subscribe();
                                drop(session_lock);

                                tokio::spawn(
                                    async move {
                                        game_loop(
                                            new_session.clone(),
                                            broadcast_sender.clone(),
                                            event_receiver,
                                        )
                                    }
                                    .await,
                                );
                                Ok((party_id, receiver, event_sender))
                            }
                            Err(err) => Err(err),
                        }
                    }
                    // tem party disponivel!!
                    Some(available_session) => {
                        let entry = sessions.get_mut(&available_session).unwrap();
                        let mut session_lock = entry.session.lock().await;
                        match session_lock.add_player(&username) {
                            Ok(_) => Ok((
                                session_lock.id,
                                entry.broadcast.subscribe(),
                                entry.event_channel.clone(),
                            )),
                            Err(err) => Err(err),
                        }
                    }
                };

            match res {
                Ok((id, b, e)) => {
                    party_id = Some(id);
                    broadcast = Some(b);
                    event_channel = Some(e);
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
        }

        let ok_signal = EventSignal::Ok(username.clone());
        let json = serde_json::to_string(&ok_signal).unwrap();

        if writer.write_all(json.as_bytes()).await.is_err()
            || writer.write_all(b"\n").await.is_err()
        {
            if let Some(entry) = sessions.lock().await.get_mut(&party_id.unwrap()) {
                entry.session.lock().await.remove_player(&username);
            }
            return;
        }

        break;
    }

    assert!(party_id.is_some());
    assert!(event_channel.is_some());
    assert!(broadcast.is_some());
    let party_id = party_id.unwrap();
    let event_channel = event_channel.unwrap();
    let mut broadcast = broadcast.unwrap();

    let _ = event_channel
        .send(GameEvent::PlayerJoined(username.clone()))
        .await;

    let mut line = String::new();

    loop {
        tokio::select! {
            result = reader.read_line(&mut line) => {
                if result.unwrap() == 0 { break; }
                let content = line.trim().to_string();
                line.clear();

                if content.starts_with("/answer ") {
                    let answer = content["/answer ".len()..].trim().to_string();
                    let _ = event_channel.send(GameEvent::PlayerAnswer { username: username.clone(), answer }).await;
                    continue;
                }
                else {
                        let msg = ChatMessage {
                            username: username.clone(),
                            content: content.clone(),
                            timestamp: get_time(),
                            message_type: MessageType::UserMessage,
                        };

                        let json = serde_json::to_string(&msg).unwrap();
                        broadcast_message(json, party_id, &sessions).await;
                }
            }

            result = broadcast.recv() => {
                if let Ok(message) = result {
                    writer.write_all(message.as_bytes()).await.unwrap();
                    writer.write_all(b"\n").await.unwrap();
                }
            }
        }
    }

    // desconectou!!
    {
        if let Some(entry) = sessions.lock().await.get_mut(&party_id) {
            entry.session.lock().await.remove_player(&username);
        }

        let leave_msg = ChatMessage {
            username: username.clone(),
            content: "left the chat".into(),
            timestamp: get_time(),
            message_type: MessageType::SystemNotification,
        };

        let leave_json = serde_json::to_string(&leave_msg).unwrap();
        broadcast_message(leave_json, party_id, &sessions).await;

        println!(
            "├─[{}] {YELLOW}'{}' disconnected.{RESET}",
            leave_msg.timestamp, username
        );
    }
}

// BROADCAST
async fn broadcast_message(msg: String, party_id: Uuid, sessions: &ServerSessions) {
    let guard = sessions.lock().await;

    if let Some(entry) = guard.get(&party_id) {
        let _ = entry.broadcast.send(format!("{}\n", msg));
    }
}

// GAME LOOP — agora funciona corretamente
// ANTES: recebia um GameSession COPIADO, ficava congelado esperando respostas que nunca chegavam.
// AGORA: recebe Arc<Mutex<GameSession>> qualquer update via /answer é visível instantaneamente
async fn game_loop(
    session: Arc<Mutex<GameSession>>,
    broadcast_channel: broadcast::Sender<String>,
    mut event_receiver: mpsc::Receiver<GameEvent>,
) {
    while let Some(event) = event_receiver.recv().await {
        match event {
            // captura o evento de um player entrar no jogo
            // se tiver 3 players, aí sim começa
            // fiz isso porque agora a thread do game_loop é iniciada diretamente ao criar uma sessão nova,
            // aí pra não deixar ela rodando com só 1 player eu coloquei essa barreira.
            GameEvent::PlayerJoined(player) => {
                let join_msg = ChatMessage {
                    username: player.clone(),
                    content: "joined the chat!".into(),
                    timestamp: get_time(),
                    message_type: MessageType::SystemNotification,
                };

                let join_json = serde_json::to_string(&join_msg).unwrap();
                let _ = broadcast_channel.send(format!("{}\n", join_json));

                println!(
                    "├─[{}] {GREEN}'{}' joined the chat!{RESET}",
                    join_msg.timestamp, player
                );

                let mut s = session.lock().await;
                if s.party.len() == 3 {
                    s.begin_game();

                    let start_msg = ChatMessage {
                        username: SYSTEM_NAME.into(),
                        content: "Aventura iniciada...".into(),
                        timestamp: get_time(),
                        message_type: MessageType::SystemNotification,
                    };

                    let json = serde_json::to_string(&start_msg).unwrap();
                    let _ = broadcast_channel.send(format!("{}\n", json));

                    // TODO: mudar pra ser Prelude
                    if let GameSceneState::Normal(scene) = &s.current_scene_state {
                        let scene_signal = EventSignal::Scene(scene.clone());
                        let scene_json = serde_json::to_string(&scene_signal).unwrap();
                        let _ = broadcast_channel.send(format!("{}\n", scene_json));
                    }
                }
                drop(s);
            }
            GameEvent::PlayerAnswer { username, answer } => {
                let mut s = session.lock().await;
                match s.update(GameEvent::PlayerAnswer { username, answer }) {
                    UpdateResult::Advance(feedback) => {
                        let msg = ChatMessage {
                            username: SYSTEM_NAME.into(),
                            content: feedback,
                            timestamp: get_time(),
                            message_type: MessageType::SystemNotification,
                        };

                        let json = serde_json::to_string(&msg).unwrap();
                        let _ = broadcast_channel.send(format!("{}\n", json));
                        // Avançar turno
                    }
                    UpdateResult::Continue => {
                        // Não faz nada kkkk
                    }
                    UpdateResult::EndGame => {
                        let end_game_msg = "Andando pelos corredores do IMD, vocês recebem uma notificação no terminal. Quando o abrem, leem a seguinte mensagem: \n 'Caros ajudantes, vocês se provaram ineficientes para a tarefa a qual lhes foi passada. Infelizmente, lhes falta conhecimento do nosso sistema para que consigam nos ajudar. Desejo que prosperem no seu desenvolvimento enquanto programadores e em outra vida sejam capazes de me ajudar. \nCrab Guardian'. Vocês saem cabisbaixos pela entrada do IMD sabendo que falharam na missão, esperando que outras pessoas mais experientes sejam capazes de consertar este caos.".into();
                        let _ = broadcast_channel.send(end_game_msg);
                        break;
                    }
                }
            }
        }
    }
}

async fn end_game(session: Arc<Mutex<GameSession>>, sender: Sender<String>) {}
