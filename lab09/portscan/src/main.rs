use std::env;
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::time::Duration;

use futures::executor::block_on;
use futures::future::join_all;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 4 {
        panic!("Usage:    ./portscan <address> <from> <to>");
    }
    let (address, from_port, to_port) = (
        &args[1],
        args[2].parse::<u16>().expect("port expected"),
        args[3].parse::<u16>().expect("port expected"),
    );
    block_on(async {
        join_all((from_port..to_port).into_iter().map(|port| {
            let addr = format!("{}:{}", address, port);
            async move {
                let addrs = addr
                    .to_socket_addrs()
                    .expect("addr expected")
                    .collect::<Vec<SocketAddr>>();
                for addr in addrs {
                    match TcpStream::connect_timeout(&addr, Duration::from_millis(500)) {
                        Ok(_) => {
                            println!("{} available", addr);
                        }
                        Err(_) => {}
                    }
                }
            }
        }))
        .await;
    });
}
