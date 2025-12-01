use cursive::views::Dialog;
use cursive::{
    views::{EditView, TextView},
    Cursive,
};
use cursive::theme::{BaseColor, Color, Effect, Style};
use cursive::utils::markup::StyledString;
use rustbreak::frontend::tui;
use rustbreak::{
    client::{
        add_scroll_callbacks, check_scroll_position, enable_auto_scroll, scroll_to_bottom,
        ScrollState,
    },
    common::{
        formatting::*,
        messages::{ChatMessage, EventSignal, MessageType},
        shared::*,
    },
    frontend::tui::make_header,
};
use std::{error::Error, sync::Arc, time::Duration};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{tcp::OwnedWriteHalf, TcpStream},
    sync::Mutex,
};

struct ClientData {
    scroll_state: ScrollState,
    writer: Arc<Mutex<OwnedWriteHalf>>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Initialize Cursive TUI and load theme
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

    siv.set_user_data(ClientData {
        scroll_state: ScrollState::new(),
        writer: Arc::clone(&writer),
    });

    // Builds TUI structure as layer stack
    // Please read the documentation before adding new layers
    tui::build_tui(&mut siv, send_message);

    add_scroll_callbacks(&mut siv);

    let reader = BufReader::new(reader);
    let mut lines = reader.lines();
    let sink = siv.cb_sink().clone();

    // Async task to handle incoming messages
    tokio::spawn(async move {
        'main_loop: while let Ok(Some(line)) = lines.next_line().await {
            // The received message json object is converted back to a `ChatMessage`
            if let Ok(msg) = serde_json::from_str::<ChatMessage>(&line) {
                let formatted_msg = match msg.message_type {
                    MessageType::UserMessage => {
                        StyledString::plain(format!(
                            "┌─[{}]\n└─ {} => {}\n",
                            msg.timestamp, msg.username, msg.content
                        ))
                    },
                    MessageType::SystemNotification => {
                        if msg.username == "ERROR" {
                            StyledString::styled(
                                format!("\n[ERROR: {}]\n", msg.content),
                                Style::from(Color::Dark(BaseColor::Red)).combine(Effect::Bold)
                            )
                        } else {
                            StyledString::plain(format!("\n[{}]\n", msg.content))
                        }
                    }
                };

                // This writes the message in the chat
                match msg.message_type {
                    // TODO: Create a third `message_type` representing game scenarios/events.
                    // Only those messages should be printed with a typing effect.
                    MessageType::UserMessage => {
                        if sink
                            .send(Box::new(move |siv: &mut Cursive| {
                                print_message(siv, formatted_msg);
                            }))
                            .is_err()
                        {
                            break 'main_loop;
                        }
                    }
                    MessageType::SystemNotification => {
                        for ch in formatted_msg.chars() {
                            if sink
                                .send(Box::new(move |siv: &mut Cursive| {
                                    print_message(siv, ch.to_string());
                                }))
                                .is_err()
                            {
                                break 'main_loop;
                            }
                            std::thread::sleep(Duration::from_millis(50));
                        }
                    }
                }
            // P.s. the next 20 lines of code were incredibly painful to come up with
            // Please remember to take a break and drink some water!
            // Because I did not.
            } else if let Ok(signal) = serde_json::from_str::<EventSignal>(&line) {
                match signal {
                    EventSignal::Error(error_msg) => {
                        let _ = sink.send(Box::new(move |siv: &mut Cursive| {
                            siv.pop_layer();
                            // Error Popup
                            siv.add_layer(
                                cursive::views::Dialog::text(error_msg)
                                    .title("Login Error")
                                    .button("Try Again", |s| {
                                        s.pop_layer();
                                    }),
                            );
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

                    EventSignal::Scene(scene) => {
                        let description = scene.description.clone();
                        let code = scene.code.clone();

                        let formatted_scene = format!(
                            "\n=== Scene {} ===\n\n{}\n\nCódigo:\n{}\n\nOpções:\nA) {}\nB) {}\nC) {}\nD) {}\n",
                            scene.id,
                            description,
                            code,
                            scene.options.a,
                            scene.options.b,
                            scene.options.c,
                            scene.options.d
                        );

                        let _ = sink.send(Box::new(move |siv: &mut Cursive| {
                            // Shows scene in chat
                            siv.call_on_name("messages", |view: &mut TextView| {
                                view.append(formatted_scene);
                            });

                            scroll_to_bottom(siv);
                        }));
                    }
                    EventSignal::Shutdown => {
                        // TODO: está acontecendo algum panic! ao finalizar, consegui ver pelo debug. Não está impactando
                        // nas outras partidas, mas é algo estranho

                        // primeiro exibe uma mensagem de finalização e depois realmente quita.
                        let _ = sink
                            .send(Box::new(|siv: &mut Cursive| {
                                siv.add_layer(Dialog::info("Fim de Jogo!"));

                                let cb = siv.cb_sink().clone();

                                tokio::spawn(async move {
                                    tokio::time::sleep(Duration::from_secs(5)).await;
                                    cb.send(Box::new(|s| s.quit())).unwrap();
                                });
                            }))
                            .unwrap();
                    }
                }
            }
        }

        // The connection dropped, so let’s notify the graphical interface (Cursive) to close.
        let _ = sink.send(Box::new(|siv: &mut Cursive| {
            siv.add_layer(
                Dialog::text("A conexão com o servidor foi encerrada. \n(O servidor pode ter sido desligado ou reiniciado)")
                    .title("Desconectado")
                    .button("Sair", |s| s.quit())
            );
        }));
    });

    siv.run();
    let _ = writer.lock().await.shutdown().await;

    Ok(())
}

/// Sends a message to the server and handle client-side commands.
///
/// ### Parameters
/// - `siv`: The TUI struct from the Cursive crate;
/// - `msg`: The message to be processed/sent.
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
            check_scroll_position(siv);
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

/// Prints a message to the chat view, respecting the auto-scroll state.
///
/// This function should be called inside the Cursive event sink.
///
/// ### Parameters
/// - `siv`: The TUI struct from the Cursive crate;
/// - `msg`: The message to be printed.
fn print_message(siv: &mut Cursive, msg: String) {
    siv.call_on_name("messages", |view: &mut TextView| {
        view.append(msg);
    });

    let should_scroll = {
        if let Some(client_data) = siv.user_data::<ClientData>() {
            client_data.scroll_state.auto_scroll
        } else {
            true
        }
    };

    if should_scroll {
        scroll_to_bottom(siv);
    }
}
