use std::{
    io::{Read, Write},
    net::{SocketAddrV6, TcpStream},
    str::FromStr,
};

fn main() {
    let addr = SocketAddrV6::from_str("[::1]:8080").expect("Incorrect addr");
    let mut stream = TcpStream::connect(addr).expect("Failed to bind");

    let message = format!("Message");
    stream
        .write_all(message.as_bytes())
        .expect("Failed to send");
    stream
        .shutdown(std::net::Shutdown::Write)
        .expect("Failed to close write side");
    let mut answer = Vec::new();
    stream
        .read_to_end(&mut answer)
        .expect("Faled to read from socket");
    println!(
        "{}",
        String::from_utf8(answer).expect("Utf-8 answer expected")
    );
}
