use std::net::TcpStream;

fn scan(port: u16) -> bool {
    TcpStream::connect(format!("127.0.0.1:{}", port)).is_ok()
}

fn main() {
    for port in [22, 80, 443] {
        if scan(port) {
            println!("{} open", port);
        }
    }
}
