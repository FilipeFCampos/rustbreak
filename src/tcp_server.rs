use std::{
    io::Read,
    net::{TcpListener, TcpStream},
    sync::Arc,
    thread,
};

use crate::player::Registry;
use crate::{
    player::Player,
    utils::command_handler::{Action, ClientMessage, CommandError},
};
use parking_lot::Mutex;

pub fn run_server(addr: &str) -> std::io::Result<()> {
    let listener = TcpListener::bind(addr)?;
    let registry = Arc::new(Mutex::new(Registry::new()));

    println!("TCP server listening on {addr}");

    for conn in listener.incoming() {
        match conn {
            Ok(stream) => {
                let registry_clone = Arc::clone(&registry);
                thread::spawn(move || handle_client(stream, registry_clone));
            }
            Err(e) => eprintln!("Accept error: {}", e),
        }
    }

    Ok(())
}

fn handle_client(mut stream: TcpStream, registry: Arc<Mutex<Registry>>) {
    let peer = stream.peer_addr().unwrap();
    println!("[+] Client connected: {}", peer);

    // cria jogador
    let mut registry_locked = registry.lock();
    let id = registry_locked.add(Player::new(stream.try_clone().unwrap()));
    drop(registry_locked);

    let mut buf = [0u8; 1024];

    loop {
        match stream.read(&mut buf) {
            Ok(0) => {
                registry.lock().remove(id);
                println!("[-] Disconnected: {}", peer);
                return;
            }

            Ok(n) => {
                let msg = ClientMessage {
                    client_id: id,
                    content: String::from_utf8_lossy(&buf[..n]).to_string(),
                };
                println!("{} says: {}", peer, msg.content.trim());
                let answer = handle_messages(&msg);
                if !answer.is_empty() {
                    println!("Server says: {}", answer.trim())
                };

                registry.lock().broadcast(&answer);
            }

            Err(_) => {
                registry.lock().remove(id);
                println!("[-] Erro de leitura: {}", peer);
                return;
            }
        }
    }
}

fn handle_messages(msg: &ClientMessage) -> String {
    match msg.eval() {
        Ok(action) => match action {
            Action::Talk(answer) => answer,
            _ => String::from("Action not implemented"),
        },
        Err(error) => match error {
            CommandError::InvalidCommand => {
                let answer = format!("Typed command doesn't exists");
                println!(
                    "ERROR: {answer}\n -> sender: {}\n -> message: {}",
                    msg.client_id, msg.content
                );
                answer
            }
            CommandError::MissingArguments => {
                let answer = format!("Typed command is missing arguments");
                println!(
                    "ERROR: {answer}\n -> sender: {}\n -> message: {}",
                    msg.client_id, msg.content
                );
                answer
            }
        },
    }
}
