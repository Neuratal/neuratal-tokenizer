//! Normalization policy and processor.
//!
//! Default policy is strictly `Normalization::None` (identity). Neuratal Tokenizer guarantees
//! exact byte-for-byte reversibility when normalization is disabled.
//!
//! The non-default modes use the Unicode Normalization Algorithm (UAX #15)
//! via the `unicode-normalization` crate and may be lossy with respect to the
//! original byte sequence (recomposition/decomposition alters the input). The
//! raw input can still be encoded; however, decoding yields the *normalized*
//! sequence, which may differ from the original input bytes.

use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use unicode_normalization::UnicodeNormalization;

/// Configurable Unicode normalization modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Normalization {
    /// Strict identity: preserves all input bytes, whitespace, case, accents, and invisible characters.
    #[default]
    None,
    /// Unicode Normalization Form C (Canonical Decomposition, followed by Canonical Composition).
    Nfc,
    /// Unicode Normalization Form D (Canonical Decomposition).
    Nfd,
    /// Unicode Normalization Form KC (Compatibility Decomposition, followed by Canonical Composition).
    /// CAUTION: Destructive for mathematics and code (e.g., compatibility characters fold).
    Nfkc,
    /// Unicode Normalization Form KD (Compatibility Decomposition).
    Nfkd,
}

impl Normalization {
    /// Normalizes input text according to the selected mode.
    /// In `Normalization::None` mode, no allocation occurs and a borrowed reference is returned.
    pub fn normalize<'a>(&self, text: &'a str) -> Cow<'a, str> {
        match self {
            Normalization::None => Cow::Borrowed(text),
            Normalization::Nfc => Cow::Owned(text.nfc().collect()),
            Normalization::Nfd => Cow::Owned(text.nfd().collect()),
            Normalization::Nfkc => Cow::Owned(text.nfkc().collect()),
            Normalization::Nfkd => Cow::Owned(text.nfkd().collect()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_normalization_preserves_identity() {
        let text = "Hello\tWorld\nİyi günler! ∑ ∫ √ ∞ 👨‍👩‍👧‍👦 123.456";
        let norm = Normalization::default();
        let normalized = norm.normalize(text);
        assert_eq!(normalized, text);
        assert!(matches!(normalized, Cow::Borrowed(_)));
    }

    #[test]
    fn test_nfc_composes_combining_marks() {
        // U+0065 U+0301 (e + combining acute) -> U+00E9 (é)
        let decomposed = "e\u{0301}";
        let normalized = Normalization::Nfc.normalize(decomposed);
        assert_eq!(normalized, "é");
    }

    #[test]
    fn test_nfd_decomposes_precomposed_chars() {
        // U+00E9 (é) -> U+0065 U+0301 (e + combining acute)
        let composed = "é";
        let normalized = Normalization::Nfd.normalize(composed);
        assert_eq!(normalized, "e\u{0301}");
    }

    #[test]
    fn test_nfc_nfd_roundtrip_stabile() {
        let text = "Türkçe: şimşek, ağlar, öğüt; 中文：深度学习; ∑∫√";
        let once = Normalization::Nfc.normalize(text).into_owned();
        let twice = Normalization::Nfc.normalize(&once).into_owned();
        assert_eq!(once, twice, "NFC must be idempotent");

        let nfd_once = Normalization::Nfd.normalize(text).into_owned();
        let nfd_twice = Normalization::Nfd.normalize(&nfd_once).into_owned();
        assert_eq!(nfd_once, nfd_twice, "NFD must be idempotent");
    }

    #[test]
    fn test_nfkc_folds_compatibility_characters() {
        // U+FB01 (ﬁ ligature) decomposes to "fi" under NFKC
        let ligature = "\u{FB01}";
        let normalized = Normalization::Nfkc.normalize(ligature);
        assert_eq!(normalized, "fi");
    }
}
