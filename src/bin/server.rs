use chrono::Local;
use rustbreak::common::{
    formatting::*,
    messages::{ChatMessage, EventSignal, MessageType},
    shared::*,
};
use rustbreak::game::game_scene::{GameScene, GameSceneState};
use rustbreak::game::game_session::{
    GameEvent, GameSession, UpdateResult, MAX_PLAYERS_PER_SESSION,
};
use rustbreak::handlers::server_handlers::{ServerSessions, SessionEntry};
use std::collections::HashMap;
use std::time::Duration;
use std::{error::Error, sync::Arc};
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
    // temos esses quatro dados primordiais
    let mut party_id: Option<Uuid> = None; // auto-explicativo, eh o id da sessão a qual o usuário pertence;
    // antes era mais útil, talvez possamos tirar
    let mut broadcast_receiver: Option<broadcast::Receiver<String>> = None; // canal de comunicação do servidor para todos os jogadores
    let mut broadcast_sender: Option<broadcast::Sender<String>> = None;
    let mut event_sender: Option<mpsc::Sender<GameEvent>> = None; // canal que possibilita o isolamento entre handle_connection e game_loop
    // com essa queue, handle_connection consegue empilhar eventos daquela sessão que serão lidos paralelamente pelo game_loop
    let mut event_receiver: Option<mpsc::Receiver<GameEvent>> = None;
    // enquanto o event_channel permite o handle_connection empilhar, o event_receiver é justamente o observador do game_loop
    // dessa ação, desempilhando as ações e podendo tomar decisões

    // seguir essa ideia é interessante para manter um isolamento bem massa entre as duas funções, pois antes estava aglutinado
    // e dificultava muito de tentar fazer alguma coisa. espero que se prove uma técnica legal kkkkkkk

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

        let username_trim = username.trim().to_string();
        if username_trim.is_empty() {
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

            let res: Result<
                (
                    Uuid,
                    broadcast::Sender<String>,
                    broadcast::Receiver<String>,
                    mpsc::Sender<GameEvent>,
                ),
                String,
            > = match available_id {
                // não tem party !!
                None => {
                    let (broadcast_sender, broadcast_receiver) = broadcast::channel::<String>(128);
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
                    match session_lock.add_player(&username_trim) {
                        Ok(_) => {
                            drop(session_lock);
                            let event_clone = event_sender.clone();
                            let broadcast_clone = broadcast_sender.clone();

                            tokio::spawn(
                                async move {
                                    game_loop(
                                        new_session.clone(),
                                        broadcast_sender.clone(),
                                        event_clone,
                                        event_receiver,
                                    )
                                }
                                .await,
                            );
                            Ok((party_id, broadcast_clone, broadcast_receiver, event_sender))
                        }
                        Err(err) => Err(err),
                    }
                }
                // tem party disponivel!!
                Some(available_session) => {
                    let entry = sessions.get_mut(&available_session).unwrap();
                    let mut session_lock = entry.session.lock().await;
                    match session_lock.add_player(&username_trim) {
                        Ok(_) => Ok((
                            session_lock.id,
                            entry.broadcast.clone(),
                            entry.broadcast.subscribe(),
                            entry.event_channel.clone(),
                        )),
                        Err(err) => Err(err),
                    }
                }
            };

            match res {
                Ok((id, b_sender, b_receiver, e_sender)) => {
                    party_id = Some(id);
                    broadcast_sender = Some(b_sender);
                    broadcast_receiver = Some(b_receiver);
                    event_sender = Some(e_sender);
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
        username = username_trim.to_string();

        break;
    }

    // coloquei asserts só pra garantir que podemos fazer unwrap de forma 100% segura abaixo.
    assert!(party_id.is_some());
    assert!(event_sender.is_some());
    assert!(broadcast_sender.is_some());
    assert!(broadcast_receiver.is_some());
    let party_id = party_id.unwrap();
    let event_channel = event_sender.unwrap();
    let broadcast_sender = broadcast_sender.unwrap();
    let mut broadcast_receiver = broadcast_receiver.unwrap();

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
                    let valid_options = ["a", "b", "c", "d"];
                    if !valid_options.contains(&answer.to_lowercase().as_str()) {
                        let error_msg = ChatMessage {
                            username: "ERROR".to_string(),
                            content: format!("Alternativa '{}' inválida! Por favor responda com A, B, C ou D.", answer),
                            timestamp: get_time(),
                            message_type: MessageType::SystemNotification,
                        };

                        if let Ok(json) = serde_json::to_string(&error_msg) {
                            let _ = writer.write_all(json.as_bytes()).await;
                            let _ = writer.write_all(b"\n").await;
                        }

                        continue;
                    }

                    let _ = event_channel.send(GameEvent::PlayerAnswer { username: username.clone(), answer }).await;
                    continue;
                }
                else {
                    send_user_msg(&username, content.clone(), &broadcast_sender).await;
                }
            }

            result = broadcast_receiver.recv() => {
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

            let leave_msg = format!("{} deixou o chat!", username);
            send_server_msg(leave_msg, &entry.broadcast).await;
        }

        println!(
            "├─[{}] {YELLOW}'{}' disconnected.{RESET}",
            get_time(),
            username
        );
    }
}

async fn game_loop(
    session: Arc<Mutex<GameSession>>,
    broadcast_channel: broadcast::Sender<String>,
    event_channel: mpsc::Sender<GameEvent>,
    mut event_receiver: mpsc::Receiver<GameEvent>,
) {
    while let Some(event) = event_receiver.recv().await {
        match event {
            // captura o evento de um player entrar no jogo
            // se tiver 3 players, aí sim começa
            // fiz isso porque agora a thread do game_loop é iniciada diretamente ao criar uma sessão nova,
            // aí pra não deixar ela rodando com só 1 player eu coloquei essa barreira.
            GameEvent::PlayerJoined(player) => {
                let join_msg = format!("{} entrou no chat!", player);
                send_server_msg(join_msg, &broadcast_channel).await;

                println!(
                    "├─[{}] {GREEN}'{}' joined the chat!{RESET}",
                    get_time(),
                    player
                );

                let mut s = session.lock().await;
                if s.party.len() == MAX_PLAYERS_PER_SESSION && !s.has_started {
                    s.begin_game();

                    send_server_msg("Aventura iniciada...".into(), &broadcast_channel).await;

                    // TODO: mudar pra ser Prelude
                    if let GameSceneState::Normal(scene) = &s.current_scene_state {
                        emit_scene_signal(&scene, &broadcast_channel).await;
                    }
                }
                drop(s);
            }
            GameEvent::PlayerAnswer { username, answer } => {
                let mut s = session.lock().await;
                match s.update(GameEvent::PlayerAnswer {
                    username: username.clone(),
                    answer,
                }) {
                    UpdateResult::Advance(feedback) => {
                        send_server_msg(feedback, &broadcast_channel).await;
                        let _ = event_channel.send(GameEvent::AdvanceTurn).await;
                    }
                    UpdateResult::Continue(Some(answer)) => {
                        // TODO: tratar/ignorar se a resposta não for a, b, c ou d
                        send_server_msg(
                            format!("{} escolheu a resposta {}", username, answer),
                            &broadcast_channel,
                        )
                        .await;
                    }
                    UpdateResult::Continue(None) => {}
                    UpdateResult::GameOver(error_msg) => {
                        game_over(error_msg, &broadcast_channel).await;
                        break;
                    }
                }
            }
            GameEvent::AdvanceTurn => {
                let mut s = session.lock().await;
                s.next_scene();
                if let GameSceneState::Normal(scene) = &s.current_scene_state {
                    emit_scene_signal(scene, &broadcast_channel).await;
                }
                drop(s);
            }
        }
    }
}

async fn game_over(error_msg_scene: String, broadcast: &broadcast::Sender<String>) {
    send_server_msg(error_msg_scene, &broadcast).await;
    tokio::time::sleep(Duration::from_secs(2)).await;

    let end_game_msg = "Andando pelos corredores do IMD, vocês recebem uma notificação no terminal. Quando o abrem, leem a seguinte mensagem: \n 'Caros ajudantes, vocês se provaram ineficientes para a tarefa a qual lhes foi passada. Infelizmente, lhes falta conhecimento do nosso sistema para que consigam nos ajudar. Desejo que prosperem no seu desenvolvimento enquanto programadores e em outra vida sejam capazes de me ajudar. \nCrab Guardian'. Vocês saem cabisbaixos pela entrada do IMD sabendo que falharam na missão, esperando que outras pessoas mais experientes sejam capazes de consertar este caos.".into();
    send_server_msg(end_game_msg, &broadcast).await;

    let shutdown_signal = EventSignal::Shutdown;
    let json = serde_json::to_string(&shutdown_signal).unwrap();
    let _ = broadcast.send(json);
}

async fn emit_scene_signal(scene: &GameScene, broadcast: &broadcast::Sender<String>) {
    let scene_signal = EventSignal::Scene(scene.clone());
    let scene_json = serde_json::to_string(&scene_signal).unwrap();
    let _ = broadcast.send(format!("{}\n", scene_json));
}

async fn send_server_msg(msg: String, broadcast: &broadcast::Sender<String>) {
    let msg = ChatMessage {
        username: SYSTEM_NAME.into(),
        content: msg,
        timestamp: get_time(),
        message_type: MessageType::SystemNotification,
    };
    let msg = serde_json::to_string(&msg).unwrap();
    let _ = broadcast.send(msg);
}

async fn send_user_msg(username: &String, msg: String, broadcast: &broadcast::Sender<String>) {
    let msg = ChatMessage {
        username: username.clone(),
        content: msg,
        timestamp: get_time(),
        message_type: MessageType::UserMessage,
    };

    let msg = serde_json::to_string(&msg).unwrap();
    let _ = broadcast.send(msg);
}
