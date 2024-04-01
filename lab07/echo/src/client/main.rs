use chrono::prelude::*;
use std::{net::UdpSocket, time::Duration};

const MAX_DATA_LENGTH: usize = 65507;

fn main() {
    let socket = UdpSocket::bind("127.0.0.1:0").expect("Failed to bind");
    socket
        .set_read_timeout(Some(Duration::from_secs(1)))
        .expect("set_read_timeout failed");

    let mut buf = [0; MAX_DATA_LENGTH];

    for i in 1..=10 {
        let stime = Utc::now().time();
        let message = format!("Ping {i} {stime}");
        socket
            .send_to(message.as_bytes(), "127.0.0.1:8080")
            .expect("Failed to send");
        match socket.recv_from(&mut buf) {
            Ok((size, _)) => {
                let ftime = Utc::now().time();
                println!("Server response: {}", std::str::from_utf8(&buf[..size]).expect("UTF-8 expected"));
                println!("RTT: {:?}", (ftime - stime).to_std().expect("Positive time expected"));
            }
            Err(_) => {
                println!("Request timed out")
            }
        }
    }
}
