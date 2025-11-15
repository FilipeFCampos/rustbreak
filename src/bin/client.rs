use rustbreak::telnet_client::{connect, send_message, read_message};
use std::io::{self, Write};

fn main() {
    let mut tel = connect("127.0.0.1:6000");

    println!("Connected! Type messages:");

    loop {
        print!("> ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();

        send_message(&mut tel, &input);

        if let Some(resp) = read_message(&mut tel) {
            println!("[server] {resp}");
        }
    }
}
