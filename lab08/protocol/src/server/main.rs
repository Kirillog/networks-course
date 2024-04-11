use protocol::Socket;
use log::info;

const MAX_DATA_LENGTH: usize = 65507;

fn main() {
    env_logger::init();
    let socket = Socket::new("127.0.0.1:8080");
    let mut buf = [0u8; MAX_DATA_LENGTH];
    let (size, addr) = socket.recv_from(&mut buf).expect("Failed to receive");
    let content = std::str::from_utf8(&buf[..size]).expect("UTF-8 string expected");
    info!("Get following file content: {}", content);
    socket
        .send_to(content.as_bytes(), addr)
        .expect("Failed to send");
}
