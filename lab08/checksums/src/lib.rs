use std::ops::Not;

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

pub fn validate_checksum(bytes: &[u8], checksum: u16) -> bool {
    calc_sum(bytes).wrapping_add(checksum).count_ones() == 16
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple() {
        assert_eq!(calc_checksum(&[0x01, 0x00, 0x02, 0x00]), 0xfffc);
        assert!(validate_checksum(&[0x01, 0x00, 0x02, 0x00], 0xfffc));
    }

    #[test]
    fn odd_len() {
        assert_eq!(calc_checksum(&[0x01, 0x00, 0x02, 0x00, 0x03]), 0xfff9);
        assert!(validate_checksum(&[0x01, 0x00, 0x02, 0x00, 0x03], 0xfff9));
    }

    #[test]
    fn overflow_u8() {
        assert_eq!(calc_checksum(&[0xff, 0x00, 0xff, 0x00]), 0xfe01);
        assert!(validate_checksum(&[0xff, 0x00, 0xff, 0x00], 0xfe01));
    }

    #[test]
    fn overflow_u16() {
        assert_eq!(calc_checksum(&[0xff, 0xff, 0xff, 0xff]), 0x0001);
        assert!(validate_checksum(&[0xff, 0xff, 0xff, 0xff], 0x0001));
    }

    #[test]
    fn negate() {
        assert!(!validate_checksum(&[0x01, 0x00, 0x03, 0x00], 0xfffc));
        assert!(!validate_checksum(&[0x01, 0x01, 0x02, 0x00, 0x03], 0xfff9));
    }
}
