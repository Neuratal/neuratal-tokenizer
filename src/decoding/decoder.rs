//! Deterministic byte-level BPE decoder.
//!
//! Reconstructs original byte sequences before performing UTF-8 validation.
//! Provides two independent decoding APIs:
//! - `decode_bytes`: Reconstructs raw bytes (never fails on invalid UTF-8).
//! - `decode_text`: Reconstructs UTF-8 string with strict validation (no silent replacement).

use crate::core::special_tokens::SpecialTokenRegistry;
use crate::core::vocabulary::{TokenId, Vocabulary};
use crate::encoding::encoder::TokenizerError;

/// Configuration options for the decoding process.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DecodeOptions {
    /// If true, registered special tokens are omitted from the decoded output.
    pub skip_special_tokens: bool,
}

/// Decodes a sequence of token IDs into a contiguous byte vector.
///
/// Accumulates exact bytes for every token. Never fails due to UTF-8 invalidity.
/// Returns an error only if an unknown token ID is encountered.
pub fn decode_bytes(
    tokens: &[TokenId],
    vocab: &Vocabulary,
    special_registry: &SpecialTokenRegistry,
    options: &DecodeOptions,
) -> Result<Vec<u8>, TokenizerError> {
    let mut output = Vec::with_capacity(tokens.len() * 2);

    for &id in tokens {
        if special_registry.is_special(id) {
            if options.skip_special_tokens {
                continue;
            }
            if let Some(content) = special_registry.get_content(id) {
                output.extend_from_slice(content.as_bytes());
                continue;
            }
        }

        match vocab.get_bytes(id) {
            Some(bytes) => output.extend_from_slice(bytes),
            None => {
                return Err(TokenizerError::DecodingError(format!(
                    "Cannot decode: token ID {} does not exist in vocabulary (size: {})",
                    id,
                    vocab.len()
                )));
            }
        }
    }

    Ok(output)
}

/// Decodes a sequence of token IDs into a verified UTF-8 string.
///
/// Reconstructs the complete byte stream first, then validates UTF-8 strictly.
/// Returns `TokenizerError::InvalidUtf8` if the reconstructed byte sequence is not valid UTF-8.
pub fn decode_text(
    tokens: &[TokenId],
    vocab: &Vocabulary,
    special_registry: &SpecialTokenRegistry,
    options: &DecodeOptions,
) -> Result<String, TokenizerError> {
    let raw_bytes = decode_bytes(tokens, vocab, special_registry, options)?;

    String::from_utf8(raw_bytes).map_err(|e| {
        TokenizerError::InvalidUtf8(format!(
            "Decoded byte sequence is not valid UTF-8 at byte position {}: {}",
            e.utf8_error().valid_up_to(),
            e
        ))
    })
}

/// Decodes a sequence of token IDs into a UTF-8 string, replacing invalid sequences with U+FFFD.
///
/// Use this only when lossy recovery of damaged streams is explicitly desired.
pub fn decode_text_lossy(
    tokens: &[TokenId],
    vocab: &Vocabulary,
    special_registry: &SpecialTokenRegistry,
    options: &DecodeOptions,
) -> Result<String, TokenizerError> {
    let raw_bytes = decode_bytes(tokens, vocab, special_registry, options)?;
    Ok(String::from_utf8_lossy(&raw_bytes).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_bytes_exact() {
        let vocab = Vocabulary::new_with_base_bytes();
        let special = SpecialTokenRegistry::new();
        let options = DecodeOptions::default();

        let input_bytes = &[0x00, 0xFF, b'a', b'b', b'c'];
        let tokens: Vec<TokenId> = input_bytes.iter().map(|&b| b as TokenId).collect();

        let decoded = decode_bytes(&tokens, &vocab, &special, &options).unwrap();
        assert_eq!(decoded, input_bytes);
    }

    #[test]
    fn test_decode_invalid_utf8_fails_text_decoding() {
        let vocab = Vocabulary::new_with_base_bytes();
        let special = SpecialTokenRegistry::new();
        let options = DecodeOptions::default();

        // 0xFF 0xFE is invalid UTF-8
        let tokens = vec![255, 254];
        let bytes_res = decode_bytes(&tokens, &vocab, &special, &options);
        assert!(bytes_res.is_ok());
        assert_eq!(bytes_res.unwrap(), vec![0xFF, 0xFE]);

        let text_res = decode_text(&tokens, &vocab, &special, &options);
        assert!(text_res.is_err());
        assert!(matches!(
            text_res.unwrap_err(),
            TokenizerError::InvalidUtf8(_)
        ));
    }
}
