use cursive::{
    Cursive,
    views::{EditView, NamedView, ScrollView, TextView},
};
use rustbreak::common::{
    formatting::*,
    messages::{ChatMessage, MessageType},
    shared::*,
};
use rustbreak::frontend::tui;
use std::{env, error::Error, sync::Arc};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{TcpStream, tcp::OwnedWriteHalf},
    sync::Mutex,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let username = env::args().nth(1).expect(&format!(
        "{RED}Please provide a username as argument.{RESET}"
    ));

    let mut siv = cursive::default();

    // Handles TUI
    tui::handle_tui(&mut siv, username.clone(), send_message);

    // Handle connecting with the server
    let stream = TcpStream::connect(format!("{ADDRESS}:{PORT}"))
        .await
        .expect(&format!(
            "{RED}ERROR: Unable to connect to server. Maybe the server is offline?\n{RESET}details"
        ));

    let (reader, mut writer) = stream.into_split();
    writer.write_all(format!("{username}\n").as_bytes()).await?;

    let writer = Arc::new(Mutex::new(writer));
    let writer_clone = Arc::clone(&writer);

    siv.set_user_data(writer);
    let reader = BufReader::new(reader);
    let mut lines = reader.lines();
    let sink = siv.cb_sink().clone();

    // Async task to handle incoming messages
    tokio::spawn(async move {
        while let Ok(Some(line)) = lines.next_line().await {
            // The received message json object is converted back to a `ChatMessage`
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

                // This writes the message in the chat
                if sink
                    .send(Box::new(move |siv: &mut Cursive| {
                        siv.call_on_name("messages", |view: &mut TextView| {
                            view.append(formatted_msg);
                        });

                        // Appends the message and forces the view to scroll down to the latest entry
                        siv.call_on_name(
                            "chat_scroll",
                            |view: &mut ScrollView<NamedView<TextView>>| {
                                view.scroll_to_bottom();
                            },
                        );
                    }))
                    .is_err()
                {
                    break;
                }
            }
        }

        // The connection dropped, so let’s notify the graphical interface (Cursive) to close.
        let _ = sink.send(Box::new(|siv: &mut Cursive| {
            siv.quit();
        }));
    });

    siv.run();
    let _ = writer_clone.lock().await.shutdown().await;

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
                    /quit - Exit chat\n\n",
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
            return;
        }
        "/quit" => {
            siv.quit();
            return;
        }
        _ => {}
    }

    let writer = siv
        .user_data::<Arc<Mutex<OwnedWriteHalf>>>()
        .unwrap()
        .clone();

    tokio::spawn(async move {
        let _ = writer
            .lock()
            .await
            .write_all(format!("{msg}\n").as_bytes())
            .await;
    });

    siv.call_on_name("input", |view: &mut EditView| {
        view.set_content("");
    });
}
