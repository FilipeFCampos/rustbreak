use rustbreak::telnet_client::{connect, read_message, send_message};
use std::io::{self, Write};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

const ADDRESS: &str = "127.0.0.1:6000";

fn main() {
    let mut tel = connect(ADDRESS);

    let (sender, receiver) = mpsc::channel::<String>();
    println!("Connected! Type messages (use /exit to quit):");

    // Thread for handling user input
    thread::spawn(move || {
        // TODO: Make so incoming output does not mess with message the current user is typing.
        loop {
            print!("> ");
            io::stdout().flush().unwrap();

            let mut input = String::new();
            io::stdin().read_line(&mut input).unwrap();

            let trimmed = input.trim();
            if trimmed == "/exit" {
                println!("Exiting client...");
                std::process::exit(0);
            }

            sender.send(trimmed.to_string()).unwrap();
        }
    });

    loop {
        // Handles message from the `input thread`
        if let Ok(msg) = receiver.try_recv() {
            send_message(&mut tel, &msg);
        }

        // Handles server messages (non-blocking)
        if let Some(resp) = read_message(&mut tel) {
            // TODO: Make this print the `user_id` from the client who sent the message
            println!("[server] {resp}")
        };

        thread::sleep(Duration::from_millis(10));
    }
}
