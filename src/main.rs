use rustbreak::tcp_server::run_server;

fn main() {
    println!("Starting TCP server on 0.0.0.0:6000...");
    run_server("0.0.0.0:6000").unwrap();
}
