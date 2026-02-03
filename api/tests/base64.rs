use std::io::{self, Read};
fn main() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap();
    let decoded = base64_decode(input.trim());
    print!("{}", String::from_utf8_lossy(&decoded));
}
fn base64_decode(s: &str) -> Vec<u8> {
    let alphabet = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = Vec::new();
    let bytes: Vec<u8> = s.bytes().collect();
    for chunk in bytes.chunks(4) {
        let a = alphabet.iter().position(|&c| c == chunk[0]).unwrap_or(0);
        let b = alphabet.iter().position(|&c| c == chunk[1]).unwrap_or(0);
        out.push(((a << 2) | (b >> 4)) as u8);
        if chunk.len() > 2 && chunk[2] != b'=' {
            let c = alphabet.iter().position(|&x| x == chunk[2]).unwrap_or(0);
            out.push((((b & 0xf) << 4) | (c >> 2)) as u8);
            if chunk.len() > 3 && chunk[3] != b'=' {
                let d = alphabet.iter().position(|&x| x == chunk[3]).unwrap_or(0);
                out.push((((c & 0x3) << 6) | d) as u8);
            }
        }
    }
    out
}
