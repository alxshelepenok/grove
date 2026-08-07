use crate::parse::{parse_internal, ParseMode};
use sha2::{Digest, Sha256};

pub fn checksum_of(body: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let digest = Sha256::digest(body.as_bytes());
    let mut out = String::with_capacity(64);
    for b in digest {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    out
}

pub fn verify_checksum(text: &str) -> bool {
    match parse_internal(text, ParseMode::Fixture) {
        Ok((_, body, expected)) => checksum_of(&body) == expected,
        Err(_) => false,
    }
}
