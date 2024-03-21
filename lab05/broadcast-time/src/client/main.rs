use std::{net::UdpSocket, thread, time::Duration};

const MAX_DATA_LENGTH: usize = 65507;

fn main() {
    let socket = UdpSocket::bind("0.0.0.0:9002").expect("Failed to create socket");
    socket.set_broadcast(true).unwrap();

    let mut buf = [0; MAX_DATA_LENGTH];
    loop {
        let (amt, _) = socket.recv_from(&mut buf).expect("Failed to read");
        let str_time = std::str::from_utf8(&buf[..amt]).expect("Failed to convert to string");
        println!("Time: {}", str_time);
        thread::sleep(Duration::from_secs(1));
    }
}
