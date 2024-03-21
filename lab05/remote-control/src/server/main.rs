use std::io::{Read, Write};
use std::process::Command;
use std::net::{TcpListener, TcpStream};

fn main() {
    let listener = TcpListener::bind("localhost:8080").unwrap();
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
    let command = String::from_utf8(buf).expect("Utf-8 expected");
    eprintln!("'{}' received", command);
    let output = Command::new("sh")
        .arg("-c")
        .arg(command)
        .output()
        .expect("Failed to execute process");
    let stddout = output.stdout;
    stream.write_all(&stddout).expect("Failed to write answer");
    stream.shutdown(std::net::Shutdown::Write).unwrap();
}
