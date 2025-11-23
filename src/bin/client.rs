// src/bin/client.rs
use cursive::{
    Cursive,
    views::{EditView, NamedView, ScrollView, TextView},
};
use rustbreak::{
    common::{
        formatting::*,
        messages::{ChatMessage, EventSignal, MessageType},
        shared::*,
    },
    frontend::tui::make_header,
    client::{ScrollState, add_scroll_callbacks, scroll_to_bottom, check_scroll_position, enable_auto_scroll}
};
use rustbreak::frontend::tui;
use std::{error::Error, sync::Arc};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{TcpStream, tcp::OwnedWriteHalf},
    sync::Mutex,
};

// Struct para armazenar múltiplos dados no user_data
struct ClientData {
    scroll_state: ScrollState,
    writer: Arc<Mutex<OwnedWriteHalf>>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut siv = cursive::default();
    siv.load_toml(include_str!("../frontend/assets/style.toml"))
        .unwrap();

    let stream = TcpStream::connect(format!("{ADDRESS}:{PORT}"))
        .await
        .expect(&format!(
            "{RED}ERROR: Unable to connect to server. Maybe the server is offline?\n{RESET}details"
        ));

    let (reader, writer) = stream.into_split();
    let writer = Arc::new(Mutex::new(writer));

    // Inicialize os dados do cliente
    siv.set_user_data(ClientData {
        scroll_state: ScrollState::new(),
        writer: Arc::clone(&writer),
    });

    tui::build_tui(&mut siv, send_message);

    // Adicione os callbacks de scroll
    add_scroll_callbacks(&mut siv);

    let reader = BufReader::new(reader);
    let mut lines = reader.lines();
    let sink = siv.cb_sink().clone();

    tokio::spawn(async move {
        while let Ok(Some(line)) = lines.next_line().await {
            if let Ok(msg) = serde_json::from_str::<ChatMessage>(&line) {
                let formatted_msg = match msg.message_type {
                    MessageType::UserMessage => format!(
                        "┌─[{}]\n└─ {} => {}\n",
                        msg.timestamp, msg.username, msg.content
                    ),
                    MessageType::SystemNotification => {
                        format!("\n[{}: {}]\n", msg.username, msg.content)
                    }
                };

                if sink
                    .send(Box::new(move |siv: &mut Cursive| {
                        siv.call_on_name("messages", |view: &mut TextView| {
                            view.append(formatted_msg);
                        });

                        // Verificar se deve fazer scroll automático
                        let should_scroll = {
                            if let Some(client_data) = siv.user_data::<ClientData>() {
                                client_data.scroll_state.auto_scroll
                            } else {
                                true // Por padrão, scroll automático
                            }
                        };

                        if should_scroll {
                            scroll_to_bottom(siv);
                        }
                    }))
                    .is_err()
                {
                    break;
                }

            } else if let Ok(signal) = serde_json::from_str::<EventSignal>(&line) {
                match signal {
                    EventSignal::Error(_) => {
                        let _ = sink.send(Box::new(move |siv: &mut Cursive| {
                            tui::error_popup(siv, "Username already taken.");
                        }));
                    }
                    EventSignal::Ok(name) => sink
                        .send(Box::new(move |siv: &mut Cursive| {
                            siv.pop_layer();
                            siv.pop_layer();
                            siv.call_on_name("header", |view: &mut TextView| {
                                view.set_content(make_header(name));
                            });
                        }))
                        .unwrap(),
                }
            }
        }

        let _ = sink.send(Box::new(|siv: &mut Cursive| {
            siv.quit();
        }));
    });

    siv.run();
    let _ = writer.lock().await.shutdown().await;

    Ok(())
}

fn send_message(siv: &mut Cursive, msg: String) {
    if msg.is_empty() {
        return;
    }

    match msg.as_str() {
        "/help" => {
            siv.call_on_name("messages", |view: &mut TextView| {
                view.append(
                    "\n=== Commands ===\n
                    /help - Show this message\n
                    /clear - Clear messages\n
                    /quit - Exit chat\n
                    /scrollon - Enable auto-scroll\n
                    /scrolloff - Disable auto-scroll\n\n",
                );
            });
            siv.call_on_name("input", |view: &mut EditView| {
                view.set_content("");
            });
            return;
        }
        "/clear" => {
            siv.call_on_name("messages", |view: &mut TextView| {
                view.set_content("");
            });
            siv.call_on_name("input", |view: &mut EditView| {
                view.set_content("");
            });

            // Após limpar, ative o auto_scroll
            if let Some(client_data) = siv.user_data::<ClientData>() {
                client_data.scroll_state.auto_scroll = true;
            }
            return;
        }
        "/scrollon" => {
            enable_auto_scroll(siv);
            siv.call_on_name("messages", |view: &mut TextView| {
                view.append("\n[Auto-scroll enabled]\n");
            });
            siv.call_on_name("input", |view: &mut EditView| {
                view.set_content("");
            });
            return;
        }
        "/scrolloff" => {
            check_scroll_position(siv); // Isso desativa o auto-scroll
            siv.call_on_name("messages", |view: &mut TextView| {
                view.append("\n[Auto-scroll disabled]\n");
            });
            siv.call_on_name("input", |view: &mut EditView| {
                view.set_content("");
            });
            return;
        }
        "/quit" => {
            siv.quit();
            return;
        }
        _ => {}
    }

    if let Some(client_data) = siv.user_data::<ClientData>() {
        let writer = client_data.writer.clone();
        tokio::spawn(async move {
            let _ = writer
                .lock()
                .await
                .write_all(format!("{msg}\n").as_bytes())
                .await;
        });
    }

    siv.call_on_name("input", |view: &mut EditView| {
        view.set_content("");
    });
}