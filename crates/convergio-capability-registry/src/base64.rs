//! Minimal base64-standard decoder. Kept inside the crate so we do not
//! pull in the `base64` crate for what is genuinely a one-call surface
//! (decoding 32-byte Ed25519 public keys).
//!
//! Accepts both padded (`AAAA==`) and unpadded input, ignores ASCII
//! whitespace. Rejects any other non-table byte.

const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Decode a base64-standard string into raw bytes.
pub(crate) fn decode(input: &str) -> Result<Vec<u8>, &'static str> {
    let mut lookup = [255u8; 256];
    for (i, b) in TABLE.iter().enumerate() {
        lookup[*b as usize] = i as u8;
    }
    let trimmed: Vec<u8> = input
        .bytes()
        .filter(|b| !b.is_ascii_whitespace() && *b != b'=')
        .collect();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }
    let mut out = Vec::with_capacity(trimmed.len() * 3 / 4 + 1);
    let mut buf: u32 = 0;
    let mut bits: u32 = 0;
    for b in trimmed {
        let v = lookup[b as usize];
        if v == 255 {
            return Err("non-base64 character");
        }
        buf = (buf << 6) | v as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push(((buf >> bits) & 0xFF) as u8);
        }
    }
    Ok(out)
}

/// Encode raw bytes as base64-standard with padding. Used by
/// `fixture_entry` to materialize a deterministic public key into the
/// on-the-wire JSON shape.
pub(crate) fn encode(bytes: &[u8]) -> String {
    let mut out = String::new();
    let mut buf: u32 = 0;
    let mut bits: u32 = 0;
    for b in bytes {
        buf = (buf << 8) | *b as u32;
        bits += 8;
        while bits >= 6 {
            bits -= 6;
            out.push(TABLE[((buf >> bits) & 0x3F) as usize] as char);
        }
    }
    if bits > 0 {
        out.push(TABLE[((buf << (6 - bits)) & 0x3F) as usize] as char);
    }
    while out.len() % 4 != 0 {
        out.push('=');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_random_lengths() {
        for n in 0..40usize {
            let bytes: Vec<u8> = (0..n).map(|i| (i as u8).wrapping_mul(17)).collect();
            let s = encode(&bytes);
            let back = decode(&s).unwrap();
            assert_eq!(bytes, back, "len={}", n);
        }
    }

    #[test]
    fn rejects_garbage() {
        assert!(decode("***").is_err());
    }

    #[test]
    fn ignores_whitespace() {
        let s = encode(b"hello world");
        let spaced = format!("{}\n  {}", &s[..4], &s[4..]);
        assert_eq!(decode(&spaced).unwrap(), b"hello world");
    }
}
