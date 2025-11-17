use rustbreak::telnet_client::{connect, send_message, read_message};
use std::io::{self, Write};

fn main() {
    let mut tel = connect("127.0.0.1:6000");

    println!("Connected! Type messages (use /exit to quit):");

    loop {
        print!("> ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();

        let trimmed = input.trim();
        if trimmed == "/exit" {
            println!("Exiting client...");
            break;
        }

        send_message(&mut tel, trimmed);

        if let Some(resp) = read_message(&mut tel) {
            println!("[server] {resp}");
        }
    }
}
