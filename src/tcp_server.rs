use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::Arc,
    thread,
};

use parking_lot::Mutex;
use crate::player::Player;
use crate::player::Registry;

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
                let msg = String::from_utf8_lossy(&buf[..n]).to_string();
                println!("{} says: {}", peer, msg.trim());

                registry.lock().broadcast(&msg);
            }

            Err(_) => {
                registry.lock().remove(id);
                println!("[-] Erro de leitura: {}", peer);
                return;
            }
        }
    }
}
