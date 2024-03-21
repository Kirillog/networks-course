use std::env;
use std::io::{Read, Write};
use std::net::TcpStream;
fn main() {
    let args: Vec<String> = env::args().collect();
    let command = &args[1..].join(" ");
    let mut stream = TcpStream::connect("localhost:8080").expect("Failed to connect to localhost");
    stream
        .write_all(command.as_bytes())
        .expect("Failed to write to socket");
    stream.shutdown(std::net::Shutdown::Write).expect("Failed to close write side");
    let mut answer = Vec::new();
    stream
        .read_to_end(&mut answer)
        .expect("Faled to read from socket");
    println!(
        "{}",
        String::from_utf8(answer).expect("Utf-8 answer expected")
    );
}
