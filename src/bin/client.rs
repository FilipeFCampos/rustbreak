use cursive::style::{BaseColor, Color, Effect, Style};
use cursive::utils::markup::StyledString;
use cursive::views::Dialog;
use cursive::{
    Cursive,
    views::{EditView, EnableableView, TextView, ScrollView, NamedView},
};
use rustbreak::common::messages::MessageType;
use rustbreak::frontend::tui;
use rustbreak::frontend::tui::make_header;
use rustbreak::game::game_scene::GameSceneType;
use rustbreak::{
    client::{
        ScrollState, add_scroll_callbacks, check_scroll_position, enable_auto_scroll
    },
    common::{
        formatting::*,
        messages::{ChatMessage, EventSignal},
        shared::*,
    },
};
use std::{error::Error, sync::Arc};
use tokio::sync::mpsc;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{TcpStream, tcp::OwnedWriteHalf},
    sync::Mutex,
    time::{sleep, Duration},
};

struct ClientData {
    scroll_state: ScrollState,
    writer: Arc<Mutex<OwnedWriteHalf>>,
}

// Arquitetura de Jobs para UI (evita travamentos)
enum UIJob {
    Instant(StyledString),
    Dynamic(String, u64),
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
            "{RED}ERROR: Impossível de se conectar ao servidor. Ele está offline? \n{RESET}"
        ));

    let (reader, writer) = stream.into_split();
    let writer = Arc::new(Mutex::new(writer));

    siv.set_user_data(ClientData {
        scroll_state: ScrollState::new(),
        writer: Arc::clone(&writer),
    });

    // Builds TUI structure as layer stack
    tui::build_tui(&mut siv, send_message);

    add_scroll_callbacks(&mut siv);

    let reader = BufReader::new(reader);
    let mut lines = reader.lines();
    let sink = siv.cb_sink().clone();

    // Canais para comunicação entre threads
    let (event_sender, mut event_receiver) = mpsc::unbounded_channel::<EventSignal>();
    let (ui_job_sender, mut ui_job_receiver) = mpsc::unbounded_channel::<UIJob>();

    // 1. THREAD DE LEITURA (Rede -> Eventos Internos)
    tokio::spawn(async move {
        while let Ok(Some(line)) = lines.next_line().await {
            if let Ok(msg) = serde_json::from_str::<ChatMessage>(&line) {
                let _ = event_sender.send(EventSignal::Message(msg));
            } else if let Ok(signal) = serde_json::from_str::<EventSignal>(&line) {
                let _ = event_sender.send(signal);
            }
        }

        // Conexão caiu
        let _ = sink.send(Box::new(|siv: &mut Cursive| {
            siv.add_layer(
                Dialog::text("A conexão com o servidor foi encerrada. \n(O servidor pode ter sido desligado ou reiniciado)")
                    .title("Desconectado")
                    .button("Sair", |s| s.quit())
            );
        }));
    });

    let sink_clone = siv.cb_sink().clone();

    // 2. THREAD DE LÓGICA (Eventos Internos -> Jobs de UI)
    tokio::spawn(async move {
        while let Some(event) = event_receiver.recv().await {
            match event {
                EventSignal::Message(msg) => match msg.message_type {
                    MessageType::UserMessage => {
                        let str = StyledString::plain(format!(
                            "┌─[{}]\n└─ {} => {}\n",
                            msg.timestamp, msg.username, msg.content
                        ));
                        ui_job_sender.send(UIJob::Instant(str)).ok();
                    }
                    MessageType::SystemNotification => {
                        let str = if msg.username == "ERROR" {
                            StyledString::styled(
                                format!("\n[ERROR: {}]\n", msg.content),
                                Style::from(Color::Dark(BaseColor::Red)).combine(Effect::Bold),
                            )
                        } else {
                            StyledString::plain(format!("\n[{}]\n", msg.content))
                        };
                        ui_job_sender.send(UIJob::Instant(str)).ok();
                    }
                },
                EventSignal::GameScene(scene) => {
                    let text: String;
                    match scene {
                        GameSceneType::Prelude(p) => text = p,
                        GameSceneType::Normal(scene) => {
                            let description = scene.description.clone();
                            let code = scene.code.clone();

                            let txt = format!(
                                "\n=== Cenário {} ===\n\n{}\n\nCódigo:\n{}\n\nOpções:\nA) {}\nB) {}\nC) {}\nD) {}",
                                scene.id, description, code, scene.options.a, scene.options.b, scene.options.c, scene.options.d
                            );
                            text = txt;
                        }
                    };
                    // Envia para o typewriter com 40ms de delay
                    ui_job_sender.send(UIJob::Dynamic(text, 40)).ok();
                }
                EventSignal::Error(err) => {
                    let _ = sink_clone.send(Box::new(move |siv: &mut Cursive| {
                        siv.pop_layer();
                        siv.add_layer(Dialog::text(err).title("Erro de Login").button(
                            "Tentar novamente",
                            |s| { s.pop_layer(); },
                        ));
                    }));
                }
                EventSignal::Shutdown => {
                    let _ = sink_clone.send(Box::new(|siv: &mut Cursive| {
                        siv.add_layer(
                            Dialog::text("Agradecemos por ter jogado Rustbreak! ;p")
                                .title("Fim do Jogo")
                                .button("Sair", |s| s.quit()),
                        );
                    })).ok();
                }
                EventSignal::Ok(name) => {
                    sink_clone.send(Box::new(move |siv: &mut Cursive| {
                        siv.pop_layer();
                        siv.pop_layer();
                        siv.call_on_name("header", |view: &mut TextView| {
                            view.set_content(make_header(name));
                        });
                    })).ok();
                }
            }
        }
    });

    let sink_clone = siv.cb_sink().clone();

    // 3. THREAD DE RENDERIZAÇÃO (Jobs de UI -> Cursive)
    tokio::spawn(async move {
        while let Some(job) = ui_job_receiver.recv().await {
            match job {
                UIJob::Instant(str) => {
                    sink_clone.send(Box::new(move |siv| {
                        siv.call_on_name("messages", |v: &mut TextView| {
                            v.append(str);
                        });
                        
                        // Scroll com estratégia StickToBottom
                        let should_scroll = if let Some(cd) = siv.user_data::<ClientData>() {
                            cd.scroll_state.auto_scroll
                        } else { true };

                        if should_scroll {
                            siv.call_on_name("chat_scroll", |view: &mut ScrollView<NamedView<TextView>>| {
                                view.set_scroll_strategy(cursive::view::ScrollStrategy::StickToBottom);
                            });
                        }
                    })).ok();
                }
                UIJob::Dynamic(text, delay_ms) => {
                    // Bloqueia Input
                    sink_clone.send(Box::new(move |siv| {
                        siv.call_on_name("chat_input", |v: &mut EnableableView<EditView>| {
                            v.disable();
                        });
                    })).ok();

                    // Typewriter effect
                    for ch in text.chars() {
                        sink_clone.send(Box::new(move |s| {
                            s.call_on_name("messages", |view: &mut TextView| {
                                view.append(ch);
                            });

                            let should_scroll = if let Some(cd) = s.user_data::<ClientData>() {
                                cd.scroll_state.auto_scroll
                            } else { true };

                            if should_scroll {
                                s.call_on_name("chat_scroll", |view: &mut ScrollView<NamedView<TextView>>| {
                                    view.set_scroll_strategy(cursive::view::ScrollStrategy::StickToBottom);
                                });
                            }
                        })).ok();

                        sleep(Duration::from_millis(delay_ms)).await;
                    }

                    // Reabilita input ao final
                    sink_clone.send(Box::new(|s| {
                        s.call_on_name("chat_input", |v: &mut EnableableView<EditView>| {
                            v.enable();
                        });
                    })).ok();
                }
            }
        }
    });

    siv.run();
    let _ = writer.lock().await.shutdown().await;

    Ok(())
}

/// Sends a message to the server and handle client-side commands.
fn send_message(siv: &mut Cursive, msg: String) {
    if msg.is_empty() {
        return;
    }

    match msg.as_str() {
        "/help" => {
            siv.call_on_name("messages", |view: &mut TextView| {
                view.append(
                    "\n=== Comandos ===\n
                    /help - Exibe esta mensagem\n
                    /clear - Limpa mensagens \n
                    /quit - Sai do jogo\n
                    /scrollon - Ativa o auto-scroll \n
                    /scrolloff - Desativa o auto-scroll\n\n",
                );
            });

            siv.call_on_name("chat_input", |view: &mut EnableableView<EditView>| {
                view.get_inner_mut().set_content("");
            });
            return;
        }
        "/clear" => {
            siv.call_on_name("messages", |view: &mut TextView| {
                view.set_content("");
            });

            siv.call_on_name("chat_input", |view: &mut EnableableView<EditView>| {
                view.get_inner_mut().set_content("");
            });

            if let Some(client_data) = siv.user_data::<ClientData>() {
                client_data.scroll_state.auto_scroll = true;
            }
            return;
        }
        "/scrollon" => {
            enable_auto_scroll(siv);
            siv.call_on_name("messages", |view: &mut TextView| {
                view.append("\n[Auto-scroll ativado]\n");
            });

            siv.call_on_name("chat_input", |view: &mut EnableableView<EditView>| {
                view.get_inner_mut().set_content("");
            });
            return;
        }
        "/scrolloff" => {
            check_scroll_position(siv);
            siv.call_on_name("messages", |view: &mut TextView| {
                view.append("\n[Auto-scroll desativado]\n");
            });

            siv.call_on_name("chat_input", |view: &mut EnableableView<EditView>| {
                view.get_inner_mut().set_content("");
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

    siv.call_on_name("chat_input", |view: &mut EnableableView<EditView>| {
        view.get_inner_mut().set_content("");
    });
}