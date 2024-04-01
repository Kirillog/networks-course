use std::net::UdpSocket;

use rand::distributions::{Bernoulli, Distribution};

const MAX_DATA_LENGTH: usize = 65507;

fn main() {
    let socket = UdpSocket::bind("127.0.0.1:8080").expect("Failed to bind");
    let bern = Bernoulli::new(0.8).unwrap();
    let mut rng = rand::thread_rng();

    let mut buf = [0; MAX_DATA_LENGTH];

    loop {
        let (size, addr) = socket.recv_from(&mut buf).expect("Failed to receive");
        let str = std::str::from_utf8(&buf[..size])
            .expect("UTF-8 string expected")
            .to_uppercase();
        if bern.sample(&mut rng) {
            socket
                .send_to(str.as_bytes(), addr)
                .expect("Failed to send");
        }
    }
}
