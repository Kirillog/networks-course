use std::{
    net::{SocketAddr, ToSocketAddrs, UdpSocket},
    time::Duration,
};

use checksums::{calc_checksum, validate_checksum};
use log::info;
use rand::distributions::{Bernoulli, Distribution, Uniform};
use zerocopy::{AsBytes, FromBytes, FromZeroes};

const MAX_DATA_LENGTH: usize = 65507;
const PACKET_PAYLOAD_SIZE: usize = 25;
const SEND_DEFAULT_TIMEOUT: Duration = Duration::from_millis(50);

#[derive(FromBytes, FromZeroes, AsBytes)]
#[repr(C, packed)]
struct Packet {
    seq_number: u32,
    checksum: u16,
    size: u8,
    payload: [u8; PACKET_PAYLOAD_SIZE],
}

impl Packet {
    fn get_seq_number(&self) -> u32 {
        self.seq_number
    }

    fn last(seq_number: u32) -> Self {
        let empty = [0u8; PACKET_PAYLOAD_SIZE];
        Packet {
            seq_number,
            checksum: calc_checksum(&empty),
            size: 0,
            payload: empty,
        }
    }
}

#[derive(FromBytes, FromZeroes, AsBytes)]
#[repr(C, packed)]
struct AccPacket(u32);

impl AccPacket {
    pub fn get_seq_number(&self) -> u32 {
        self.0
    }
}

pub struct Socket {
    udp_socket: UdpSocket,
    send_timeout: Duration,
}

impl Socket {
    pub fn new<A: ToSocketAddrs>(addr: A) -> Self {
        let udp_socket = UdpSocket::bind(addr).expect("Failed to bind");
        Socket {
            udp_socket,
            send_timeout: SEND_DEFAULT_TIMEOUT,
        }
    }

    pub fn set_timeout(&mut self, duration: Duration) {
        self.send_timeout = duration;
    }

    fn split_into_packets(buf: &[u8]) -> Vec<Packet> {
        let mut packs: Vec<Packet> = buf
            .chunks(PACKET_PAYLOAD_SIZE)
            .enumerate()
            .map(|(i, chunk)| {
                let mut payload = [0u8; PACKET_PAYLOAD_SIZE];
                payload[..chunk.len()].copy_from_slice(chunk);
                Packet {
                    seq_number: i as u32,
                    checksum: calc_checksum(chunk),
                    size: chunk.len() as u8,
                    payload,
                }
            })
            .collect();
        packs.push(Packet::last(packs.len() as u32));
        packs
    }

    pub fn send_to<A: ToSocketAddrs + Copy>(&self, buf: &[u8], addr: A) -> std::io::Result<usize> {
        self.udp_socket.set_read_timeout(Some(self.send_timeout))?;
        let mut packets = Socket::split_into_packets(buf);
        let mut sended = 0;
        let mut recv_buf = [0; MAX_DATA_LENGTH];
        let corr_dist = Bernoulli::new(0.1).unwrap();
        let unif_dist = Uniform::new(0, PACKET_PAYLOAD_SIZE);
        let mut ind: usize = 0;
        let mut corrupted_info: u8;
        for mut packet in packets.drain(..) {
            loop {
                corrupted_info = packet.payload[ind];
                if corr_dist.sample(&mut rand::thread_rng()) {
                    info!("{} packet was corrupted", packet.get_seq_number());
                    ind = unif_dist.sample(&mut rand::thread_rng());
                    corrupted_info = packet.payload[ind];
                    packet.payload[ind] = 0;
                }
                self.udp_socket.send_to(packet.as_bytes(), addr)?;
                packet.payload[ind] = corrupted_info;
                match self.udp_socket.recv_from(&mut recv_buf) {
                    Ok((_, _)) => {
                        if let Some(acc_packet) = AccPacket::ref_from_prefix(recv_buf.as_slice()) {
                            if acc_packet.0 == packet.seq_number {
                                sended += packet.size as usize;
                                info!("Sent {} packet", packet.get_seq_number());
                                break;
                            } else {
                                info!(
                                    "{} packet get, but {} packet expected",
                                    acc_packet.get_seq_number(),
                                    packet.get_seq_number()
                                );
                            }
                        }
                    }
                    Err(_) => {
                        info!("Lost packet {}", packet.get_seq_number())
                    }
                }
            }
        }
        info!("All sent");
        Ok(sended)
    }

    pub fn recv_from(&self, buf: &mut [u8]) -> std::io::Result<(usize, SocketAddr)> {
        self.udp_socket.set_read_timeout(None)?;
        let mut size = 0;
        let mut recv_buf = [0; MAX_DATA_LENGTH];
        let loss_dist = Bernoulli::new(0.3).unwrap();
        let mut seq_number = 0;
        loop {
            let (recv_size, addr) = self.udp_socket.recv_from(&mut recv_buf)?;
            if let Some(packet) = Packet::ref_from(&recv_buf[..recv_size]) {
                if loss_dist.sample(&mut rand::thread_rng()) {
                    info!("{} packet has been lost", packet.get_seq_number());
                } else if !validate_checksum(packet.payload.as_slice(), packet.checksum) {
                    info!("{} packet has been corrupted", packet.get_seq_number());
                } else if packet.seq_number < seq_number {
                    info!("{} packet has been duplicated", packet.get_seq_number());
                } else if packet.size == 0 {
                    info!("All received");
                    self.udp_socket
                        .send_to(AccPacket(packet.seq_number).as_bytes(), addr)?;
                    return Ok((size, addr));
                } else {
                    info!("Received {} packet", packet.get_seq_number());
                    buf[size..size + PACKET_PAYLOAD_SIZE]
                        .copy_from_slice(packet.payload.as_slice());
                    size += packet.size as usize;
                    seq_number = packet.seq_number + 1;
                    self.udp_socket
                        .send_to(AccPacket(packet.seq_number).as_bytes(), addr)?;
                }
            }
        }
    }
}
