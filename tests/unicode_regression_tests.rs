//! Unicode regression tests for the v0.3-v2 cycle.
//!
//! Root cause fixed: `split_structured_unicode` and
//! `split_leading_whitespace_unicode` dropped bytes when a string *started with*
//! combining marks (Devanagari vowel signs, Thai vowel signs, Arabic harakat,
//! U+0301, ...): the first non-mark character reset the run start, so the
//! leading marks disappeared from the pre-token stream entirely.
//!
//! Every test here fails on the unfixed code and passes now.

use neuratal_tokenizer::{PretokenizationMode, Tokenizer};

/// Every adversarial sample must be fully covered by the pre-tokenizer slices:
/// concatenating the slices must reproduce the input byte-for-byte.
const ADVERSARIAL: &[&str] = &[
    "\u{0301}\u{0300}\u{0327}",      // enclosing-style leading marks only
    "\u{0301}abc def\u{0301}",       // leading mark + word
    "\u{0301}\nline2",               // leading mark + newline
    "\u{0301}中文字",                // leading mark + CJK
    "\u{0E34}\u{0E34}thai",          // Thai vowel signs first
    "\u{093F}किताब",                 // Devanagari vowel sign I first
    "\u{094E}\u{093E}\u{094D}\u{093F}abc", // mixed devanagari marks first
    "\u{064E}بِسْمِ",                // Arabic fatha first
    "\u{064E}\u{0650}\u{0652}بِسْمِ ٱللَّهِ", // Arabic harakat first
    "\u{200D}zwj first",             // leading ZWJ
    "\u{FE0F}variation selector first", // leading VS16
    "\u{1F3FD}skin tone first",     // leading emoji modifier
    "\u{0903}visarga first",         // devanagari spacing mark first
    "किता\u{093E}ब",                 // interior matras intact
    "بِسْمِ ٱللَّهِ ٱلرَّحْمَٰنِ",   // arabic harakat everywhere
    "👨\u{200D}👩\u{200D}👧\u{200D}👦", // ZWJ sequence
    "👍🏽 👍🏿 👋🏻",                  // skin tones
    "🇹🇷 🇺🇸 🇯🇵",                  // regional indicator flags
    "❤️ ✈️ ☀︎",                      // emoji + variation selectors
    "İstanbul Iğdır",                // Turkish dotted capital I
    "I\u{0307}stanbul",              // Turkish I + combining dot above (NFD-ish)
    "e\u{0301}\u{0327} a\u{0300}\u{0316}", // stacked diacritics
    "1\u{20DD} e\u{20E0} a\u{20E3}", // enclosing marks
    "αβγ中文δбгд",                    // mixed scripts
    "Hello世界123，テスト。",          // mixed script punctuation
    "a\nb\r\nc\td  \n\n\tef",        // newlines and tabs
    "a\u{0085}b\u{00A0}c\u{2028}d\u{2029}e\u{202F}f\u{2007}g\u{3000}h", // unusual whitespace
    "co\u{00AD}operate",             // soft hyphen
    "a\u{2060}b",                    // word joiner
    "\u{10FFFF}\u{F0000}\u{0378}\u{E000}", // rare/unassigned/PUA scalars
];

const ALL_MODES: &[PretokenizationMode] = &[
    PretokenizationMode::RawByte,
    PretokenizationMode::ConservativeUnicode,
    PretokenizationMode::ScriptAndDigitAware,
    PretokenizationMode::LeadingWhitespaceUnicode,
    PretokenizationMode::ScriptTransitionAware,
    PretokenizationMode::UnicodeHybrid,
    PretokenizationMode::CodeAware,
    PretokenizationMode::MathSymbolAware,
    PretokenizationMode::MathSymbolAwareCjk2,
    PretokenizationMode::MathSymbolAwareCjk4,
    PretokenizationMode::MathSymbolAwareCjk8,
    PretokenizationMode::MathSymbolAwareCjkHan4,
];

#[test]
fn pretokenizer_slices_tile_input_all_modes() {
    for mode in ALL_MODES {
        for (i, text) in ADVERSARIAL.iter().enumerate() {
            let slices = mode.split_text(text);
            let rebuilt: String = slices.concat();
            assert_eq!(
                rebuilt, *text,
                "mode {mode:?} lost bytes on adversarial sample {i}: {text:?}"
            );
        }
    }
}

#[test]
fn leading_marks_roundtrip_text_and_bytes() {
    let tok = Tokenizer::new_base();
    for (i, text) in ADVERSARIAL.iter().enumerate() {
        let ids = tok.encode_bytes(text.as_bytes());
        let bytes = tok.decode_bytes(&ids).expect("decode_bytes must succeed");
        assert_eq!(bytes, text.as_bytes(), "byte round trip failed for sample {i}");

        let ids = tok.encode_text(text).expect("encode_text must succeed");
        let s = tok.decode_text(&ids).expect("decode_text must succeed");
        assert_eq!(&s, text, "text round trip failed for sample {i}");
    }
}

#[test]
fn all_marks_only_text_survives() {
    for mode in ALL_MODES {
        let text = "\u{0301}\u{0300}\u{0327}";
        let slices = mode.split_text(text);
        let rebuilt: String = slices.concat();
        assert_eq!(rebuilt, text, "mode {mode:?} lost bytes on all-marks input");
    }
}

#[test]
fn normalization_is_not_applied_by_default() {
    let tok = Tokenizer::new_base();
    let text = "e\u{0301} combined and \u{00E9} precomposed, İ vs I\u{0307}";
    let ids = tok.encode_text(text).unwrap();
    let back = tok.decode_text(&ids).unwrap();
    assert_eq!(back, text, "default pipeline must not normalize");
}

#[test]
fn normalization_modes_are_explicitly_opt_in() {
    use neuratal_tokenizer::Normalization;
    let text = "e\u{0301}";
    let nfc = Normalization::Nfc.normalize(text);
    assert_eq!(nfc, "\u{00E9}");
    let nfd = Normalization::Nfd.normalize("\u{00E9}");
    assert_eq!(nfd, "e\u{0301}");
    // Default stays identity
    assert_eq!(Normalization::None.normalize(text), text);
}

#[test]
fn empty_and_whitespace_edges() {
    let tok = Tokenizer::new_base();
    for text in ["", " ", "\n", "\t", "\u{2028}\u{2029}", "\u{FEFF}"] {
        let ids = tok.encode_text(text).unwrap();
        let back = tok.decode_text(&ids).unwrap();
        assert_eq!(back, text);
        let ids = tok.encode_bytes(text.as_bytes());
        assert_eq!(tok.decode_bytes(&ids).unwrap(), text.as_bytes());
    }
}
