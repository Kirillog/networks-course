pub fn crc32(bytes: &[u8]) -> u32 {
    let mut crc_table = [0u32; 256];
    let mut crc;
    for i in 0usize..256 {
        crc = i as u32;
        for _ in 0..8 {
            crc = if crc & 1 > 0 {
                (crc >> 1) ^ 0xEDB88320u32
            } else {
                crc >> 1
            };
        }
        crc_table[i] = crc;
    }

    crc = 0xFFFFFFFFu32;

    for i in 0..bytes.len() {
        crc = crc_table[((crc ^ bytes[i] as u32) & 0xFFu32) as usize] ^ (crc >> 8);
    }
    return crc ^ 0xFFFFFFFFu32;
}

#[cfg(test)]
mod tests {
    use std::fs;

    use rand::{
        distributions::{Bernoulli, Uniform},
        Rng,
    };

    use super::*;

    #[test]
    fn trivial() {
        let str = "";
        assert_eq!(crc32(str.as_bytes()), 0x0);
    }

    #[test]
    fn short() {
        let str = "123456789";
        assert_eq!(crc32(str.as_bytes()), 0xcbf43926);
    }

    #[test]
    fn long() {
        let str = "The quick brown fox jumps over the lazy dog";
        assert_eq!(crc32(str.as_bytes()), 0x414FA339);
    }

    fn distort_packet(packet: &[u8], bit_number: usize) -> Vec<u8> {
        let mut packet = packet.to_vec();
        packet[bit_number >> 3] ^= 1 << (bit_number & 0x7usize);
        packet
    }

    #[test]
    fn main() {
        let data = fs::read("test/test.txt").expect("test file expected");
        let mut rng = rand::thread_rng();
        let d = Bernoulli::new(0.3).unwrap();
        data.as_slice()
            .chunks(5)
            .enumerate()
            .for_each(|(i, packet)| {
                let uni = Uniform::new(0usize, packet.len() * 8);
                println!(
                    "Payload:  {:?}",
                    std::str::from_utf8(packet).expect("UTF-8 expected")
                );
                println!("Encoded:  {:#x?}", packet);
                println!("Crc32:    {:#2x?}", crc32(packet));
                if rng.sample(d) {
                    let distort_packet = distort_packet(packet, rng.sample(uni));
                    println!("Catch distortion in {:?}", i);
                    assert_ne!(crc32(&packet), crc32(&distort_packet));
                }
                println!("");
            })
    }
}
