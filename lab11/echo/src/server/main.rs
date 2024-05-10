use std::{
    io::{Read, Write},
    net::{SocketAddrV6, TcpListener, TcpStream},
    str::FromStr,
};

fn main() {
    let addr = SocketAddrV6::from_str("[::1]:8080").expect("Incorrect addr");

    let listener = TcpListener::bind(addr).expect("Failed to bind");

    for stream in listener.incoming() {
        let stream = stream.unwrap();
        handle_connection(stream);
    }
}

fn handle_connection(mut stream: TcpStream) {
    let mut buf = Vec::new();
    stream
        .read_to_end(&mut buf)
        .expect("Failed to read request");
    let message = String::from_utf8(buf)
        .expect("Utf-8 expected")
        .to_uppercase();
    stream
        .write_all(&message.as_bytes())
        .expect("Failed to write answer");
    stream
        .shutdown(std::net::Shutdown::Write)
        .expect("Failed to close write side");
}
