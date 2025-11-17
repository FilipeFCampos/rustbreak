use telnet::Telnet;

pub fn connect(address: &str) -> Telnet {
    Telnet::connect(address, 256).expect("Failed to connect to server")
}

pub fn send_message(client: &mut Telnet, msg: &str) {
    let formatted = format!("{}\n", msg);
    client.write(formatted.as_bytes()).unwrap();
}

pub fn read_message(client: &mut Telnet) -> Option<String> {
    match client.read_nonblocking() {
        Ok(telnet::Event::Data(data)) => {
            let msg = String::from_utf8_lossy(&data[..]);
            Some(format!("{}", msg.trim()))
        }
        Ok(_) | Err(_) => None,
    }
}
