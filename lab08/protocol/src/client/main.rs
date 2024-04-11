use log::info;
use protocol::Socket;
use std::{
    env,
    fs::File,
    io::{BufReader, Read},
    path::Path,
};

const MAX_DATA_LENGTH: usize = 65507;

fn main() -> std::io::Result<()> {
    env_logger::init();
    let socket = Socket::new("127.0.0.1:0");
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        panic!("Usage:    ./client <file_name>");
    }
    let mut send_buf = [0u8; MAX_DATA_LENGTH];
    let send_size = BufReader::new(File::open(Path::new(&args[1])).expect("Failed to open file"))
        .read(&mut send_buf)?;
    socket
        .send_to(&send_buf[..send_size], "127.0.0.1:8080")
        .expect("Failed to send");
    let mut recv_buf = [0u8; MAX_DATA_LENGTH];
    let (recv_size, _) = socket.recv_from(&mut recv_buf).expect("Failed to receive");
    assert!(recv_size == send_size);
    let str = std::str::from_utf8(&recv_buf[..recv_size])
        .expect("UTF-8 string expected");
    info!("Get following file content: {}", str);
    Ok(())
}
