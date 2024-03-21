use std::{net::UdpSocket, thread, time::Duration};

use chrono::prelude::*;

fn main() {
    let socket = UdpSocket::bind("0.0.0.0:0").expect("Failed to bind for broadcast");
    socket
        .set_broadcast(true)
        .expect("Faled to set broadcast flag");
    let broadcast_addr = format!("255.255.255.255:9002");

    loop {
        let utc_time = Utc::now().to_rfc3339();
        socket
            .send_to(utc_time.as_bytes(), broadcast_addr.clone())
            .expect("Failed to send time");
        thread::sleep(Duration::from_secs(1));
    }
}
