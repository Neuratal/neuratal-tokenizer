//! Byte primitives and base byte representation.
//!
//! In Neuratal Tokenizer, every byte (0x00 through 0xFF) maps deterministically to
//! Token IDs 0 through 255. This guarantees that any arbitrary byte sequence
//! can be represented without missing or unknown tokens.

/// The total number of base byte tokens in the vocabulary.
pub const BASE_BYTE_COUNT: usize = 256;

/// Converts a raw byte value into its dedicated base token ID (0..=255).
#[inline(always)]
pub fn byte_to_token_id(b: u8) -> u32 {
    b as u32
}

/// Converts a base token ID back to its byte value if in the range 0..=255.
#[inline(always)]
pub fn token_id_to_byte(id: u32) -> Option<u8> {
    if id < BASE_BYTE_COUNT as u32 {
        Some(id as u8)
    } else {
        None
    }
}

/// Formats a byte slice into a human-readable string for inspection and debugging.
/// Printable ASCII characters are displayed as-is, while non-printable bytes or
/// whitespace characters are formatted as escape sequences (e.g., `\n`, `\t`, `\xHH`).
pub fn format_bytes_for_display(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        match b {
            b'\\' => out.push_str("\\\\"),
            b'\n' => out.push_str("\\n"),
            b'\r' => out.push_str("\\r"),
            b'\t' => out.push_str("\\t"),
            0x20..=0x7E => out.push(b as char),
            _ => {
                use std::fmt::Write;
                let _ = write!(out, "\\x{:02X}", b);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_256_bytes_bijective_mapping() {
        for b in 0u8..=255u8 {
            let id = byte_to_token_id(b);
            assert_eq!(id, b as u32, "Byte {} must map to token ID {}", b, b);
            assert_eq!(
                token_id_to_byte(id),
                Some(b),
                "Token ID {} must map back to byte {}",
                id,
                b
            );
        }
        assert_eq!(token_id_to_byte(256), None);
        assert_eq!(token_id_to_byte(1000), None);
    }

    #[test]
    fn test_format_bytes_display() {
        assert_eq!(format_bytes_for_display(b"Hello\nWorld"), "Hello\\nWorld");
        assert_eq!(format_bytes_for_display(&[0x00, 0xFF, b'A']), "\\x00\\xFFA");
    }
}
