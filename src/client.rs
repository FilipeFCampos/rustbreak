use crate::telnet_client::{connect, read_message, send_message};
use std::io::{self, Write};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

const ADDRESS: &str = "127.0.0.1:6000";

fn main() {
    let mut telnet = match connect(ADDRESS) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Failed to connect: {}", e);
            return;
        }
    };
    println!("Connected! Type messages (use /exit to quit):");

    let (sender, receiver) = mpsc::channel();
    let input_sender = sender.clone();

    thread::spawn(move || {
        loop {
            print!("> ");
            io::stdout().flush().unwrap();

            let mut input = String::new();
            io::stdin().read_line(&mut input).unwrap();

            let trimmed = input.trim();
            if trimmed == "/exit" {
                println!("Exiting client...");
                let _ = input_sender.send("/exit".to_string());
                break;
            }

            if !trimmed.is_empty() {
                let _ = input_sender.send(trimmed.to_string());
            }
        }
    });

    'main_loop: loop {
        match receiver.try_recv() {
            Ok(msg) => {
                if msg == "/exit" {
                    break 'main_loop;
                }
                if let Err(e) = send_message(&mut telnet, &msg) {
                    eprintln!("Failed to send message: {}", e);
                    break 'main_loop;
                }
            }
            Err(mpsc::TryRecvError::Disconnected) => break 'main_loop,
            Err(mpsc::TryRecvError::Empty) => {}
        }

        match read_message(&mut telnet) {
            Ok(Some(resp)) => {
                print!("\r\x1B[K");
                println!("[server] {}", resp);
                print!("> ");
                io::stdout().flush().unwrap();
            }
            Ok(None) => {} // This is where is no available message
            Err(e) => {
                eprintln!("Connection error: {}", e);
                break 'main_loop;
            }
        }
        thread::sleep(Duration::from_millis(10));
    }
    println!("Client disconnected.");
}