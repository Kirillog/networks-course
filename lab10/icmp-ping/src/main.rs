use std::{
    cmp::{max, min}, env, io::Read, net::ToSocketAddrs, ops::Not, time::Duration
};

use chrono::{TimeDelta, Utc};
use socket2::{self};

fn calc_sum(bytes: &[u8]) -> u16 {
    bytes.chunks(2).fold(0u16, |acc, item| {
        acc.wrapping_add(
            item.iter()
                .rev()
                .fold(0u16, |num, &item| num << 8 | item as u16),
        )
    })
}

pub fn calc_checksum(bytes: &[u8]) -> u16 {
    calc_sum(bytes).not()
}

fn tou16(a: u8, b: u8) -> u16 {
    (a as u16) << 8 | b as u16
}

fn to2u8(i: u16) -> (u8, u8) {
    ((i >> 8) as u8, (i & 0xFF) as u8)
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        panic!("Usage:    ./icmp-ping <address>");
    }
    let address = &args[1];

    let mut icmp_socket = socket2::Socket::new(
        socket2::Domain::IPV4,
        socket2::Type::RAW,
        Some(socket2::Protocol::ICMPV4),
    )
    .unwrap();

    let dest = format!("{}:0", address)
    .to_socket_addrs()
    .expect("addr expected")
    .next()
    .unwrap();

    icmp_socket.connect(&dest.into()).unwrap();

    icmp_socket
        .set_read_timeout(Some(Duration::from_secs(1)))
        .unwrap();

    eprintln!("{:?}", dest);

    let mut buffer = [0; 1024];
    let mut sum_rtt = TimeDelta::new(0, 0).unwrap();
    let mut min_rtt = TimeDelta::max_value();
    let mut max_rtt = TimeDelta::min_value();
    let mut succ: u16 = 0;
    for i in 0..10 {
        let stime = Utc::now().time();
        let time_str = stime.to_string();
        let payload = time_str.as_bytes();

        let mut buf = [0u8; 8 + 56];
        buf[8..8 + payload.len()].clone_from_slice(payload);
        buf[0] = 8;
        buf[5] = 1;
        (buf[6], buf[7]) = to2u8(i);
        (buf[2], buf[3]) = to2u8(calc_checksum(&buf));
        icmp_socket.send(&buf).unwrap();
        match icmp_socket.read(&mut buffer) {
            Ok(_) => {
                let ftime = Utc::now().time();
                let icmp_buf = &mut buffer[20..];
                let j = tou16(icmp_buf[6], icmp_buf[7]);
                if icmp_buf[0] == 0 && icmp_buf[1] == 0 && icmp_buf[5] == 1 && j == i {
                    let cur_rtt = ftime - stime;
                    succ += 1;
                    sum_rtt += cur_rtt;
                    min_rtt = min(min_rtt, cur_rtt);
                    max_rtt = max(max_rtt, cur_rtt);
                    println!(
                        "{:?}: min={:?}, max={:?}, avg={:?}, succ={}%",
                        dest,
                        min_rtt.to_std().unwrap(),
                        max_rtt.to_std().unwrap(),
                        (sum_rtt / succ.into()).to_std().unwrap(),
                        succ as f32 * 100. / (i + 1) as f32
                    );
                }
                std::thread::sleep(Duration::from_secs(1));
            }
            Err(_) => {
                println!("Packet has been lost");
            }
        }
    }
}
