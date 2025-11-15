use telnet::Telnet;

pub fn connect(address: &str) -> Telnet {
    Telnet::connect(address, 256).expect("Failed to connect to server")
}

pub fn send_message(client: &mut Telnet, msg: &str) {
    let formatted = format!("{}\n", msg);
    client.write(formatted.as_bytes()).unwrap();
}

pub fn read_message(client: &mut Telnet) -> Option<String> {
    match client.read() {
        Ok(event) => Some(format!("{:?}", event)),
        Err(_) => None,
    }
}
