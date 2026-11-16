//! Phase 5F property-style round-trip tests.
//!
//! Uses the deterministic XorShift64* RNG to generate reproducible pseudo-random
//! inputs and verifies `decode(encode(x)) == x` across:
//!   1. arbitrary byte sequences (binary API), including invalid UTF-8
//!   2. arbitrary Unicode strings built from a broad character pool (text API)
//!   3. a trained BPE model, not just the base tokenizer
//!
//! A property-testing framework (proptest/quickcheck) is deliberately not used to
//! avoid a large dependency; determinism via a fixed seed gives reproducible failures.

use neuratal_tokenizer::{
    CorpusManifest, CorpusManifestEntry, DeterministicRng, Normalization, PretokenizationMode,
    Tokenizer, TrainingConfig, train_from_manifest,
};
use std::fs::File;
use std::io::Write;

static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// Broad representative character pool: Latin, Turkish, Cyrillic, Greek, Arabic,
/// Hebrew, Devanagari, Thai, CJK, Hiragana, Katakana, Hangul, math symbols,
/// whitespace, digits, emoji + combining marks + joiners.
const CHAR_POOL: &[char] = &[
    'a', 'Z', 'é', 'ü', 'ş', 'ğ', 'İ', 'ı', 'ö', 'ç', 'Я', 'ж', 'Щ', 'π', 'Ω', 'Γ', 'ع', 'ب', 'ش',
    'م', 'ة', 'َ', 'ُ', 'ב', 'ש', 'ל', 'ם', 'ֶ', 'क', 'ष', '्', 'ा', 'ก', 'ไ', '์', '中', '文', '汉',
    '字', '語', '言', '한', '글', '가', '나', '∑', '∫', '√', '∞', 'π', '≤', '≠', 'Δ', 'λ', 'μ',
    '0', '9', '7', ' ', '\t', '\n', '.', ',', '!', '?', ';', ':', '(', ')', '{', '}', '[', ']',
    '"', '\'', '\\', '🚀', '👨', '👩', '👧', '👦', '‍', '🇮', '🇹', '🇷', '🇺', '🇸', '\u{0301}',
    '\u{036F}', '\u{0591}', '\u{064B}', '\u{0E31}', // combining marks
    '\u{200D}', '\u{200C}', // ZWJ / ZWNJ
];

fn random_bytes(rng: &mut DeterministicRng, max_len: usize) -> Vec<u8> {
    let len = (rng.next_u64() as usize) % max_len;
    (0..len).map(|_| (rng.next_u64() & 0xFF) as u8).collect()
}

fn random_string(rng: &mut DeterministicRng, max_len: usize) -> String {
    let len = (rng.next_u64() as usize) % max_len;
    (0..len)
        .map(|_| CHAR_POOL[(rng.next_u64() as usize) % CHAR_POOL.len()])
        .collect()
}

fn train_small_model() -> Tokenizer {
    let id = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let corpus_path =
        std::env::temp_dir().join(format!("property_corpus_{}_{}.txt", std::process::id(), id));
    {
        let mut f = File::create(&corpus_path).unwrap();
        for _ in 0..6 {
            writeln!(f, "the quick brown fox jumps over the lazy dog repeatedly").unwrap();
            writeln!(f, "machine learning models love large amounts of text data").unwrap();
            writeln!(
                f,
                "numbers 1234567890 and symbols ∫∫∫ ∑∑∑ sqrt(2)=1.41421356"
            )
            .unwrap();
            writeln!(
                f,
                "emoji sequences 👨‍👩‍👧‍👦 🚀🇹🇷 and combining accents e\u{0301} a\u{0300}"
            )
            .unwrap();
            writeln!(
                f,
                "multilingual: Türkçe Русский Ελληνικά العربية עברית 中文 ไทย"
            )
            .unwrap();
        }
    }

    let mut manifest = CorpusManifest::new(42);
    manifest.add_entry(CorpusManifestEntry {
        path: corpus_path.clone(),
        weight: 1.0,
        language: None,
        domain: None,
        text_field: None,
    });

    let config = TrainingConfig {
        target_vocab_size: 320,
        min_frequency: 2,
        pretokenizer: PretokenizationMode::ConservativeUnicode,
        normalization: Normalization::None,
        special_tokens: vec!["<|bos|>".to_string(), "<|eos|>".to_string()],
        seed: 42,
    };

    let (tok, _) = train_from_manifest(&manifest, &config).unwrap();
    let _ = std::fs::remove_file(corpus_path);
    tok
}

#[test]
fn property_binary_roundtrip_arbitrary_bytes() {
    let base = Tokenizer::new_base();
    let mut rng = DeterministicRng::new(0x5EED_0001);

    for i in 0..2000 {
        let bytes = random_bytes(&mut rng, 384);
        let encoded = base.encode_bytes(&bytes);
        let decoded = base.decode_bytes(&encoded).expect("decode_bytes failed");
        assert_eq!(
            decoded,
            bytes,
            "binary round-trip failed on case {} ({:?}...)",
            i,
            &bytes[..bytes.len().min(16)]
        );
    }
}

#[test]
fn property_binary_roundtrip_arbitrary_bytes_trained_model() {
    let trained = train_small_model();
    let mut rng = DeterministicRng::new(0x5EED_0002);

    for i in 0..500 {
        let bytes = random_bytes(&mut rng, 256);
        let encoded = trained.encode_bytes(&bytes);
        let decoded = trained.decode_bytes(&encoded).expect("decode_bytes failed");
        assert_eq!(
            decoded, bytes,
            "trained-model binary round-trip failed on case {}",
            i
        );
    }
}

#[test]
fn property_text_roundtrip_valid_utf8() {
    let base = Tokenizer::new_base();
    let mut rng = DeterministicRng::new(0x5EED_0003);

    for i in 0..2000 {
        let text = random_string(&mut rng, 96);
        let encoded = base.encode_text(&text).expect("encode_text failed");
        let decoded = base.decode_text(&encoded).expect("decode_text failed");
        assert_eq!(
            decoded, text,
            "text round-trip failed on case {}: {:?}",
            i, text
        );
    }
}

#[test]
fn property_text_roundtrip_trained_model() {
    let trained = train_small_model();
    let mut rng = DeterministicRng::new(0x5EED_0004);

    for i in 0..1000 {
        let text = random_string(&mut rng, 64);
        let encoded = trained.encode_text(&text).expect("encode_text failed");
        let decoded = trained.decode_text(&encoded).expect("decode_text failed");
        assert_eq!(
            decoded, text,
            "trained-model text round-trip failed on case {}: {:?}",
            i, text
        );
    }
}

#[test]
fn property_empty_and_single_byte_inputs() {
    let tok = Tokenizer::new_base();

    let encoded = tok.encode_bytes(b"");
    assert!(encoded.is_empty());
    assert_eq!(tok.decode_bytes(&encoded).unwrap(), b"");

    for b in 0u8..=255 {
        let encoded = tok.encode_bytes(&[b]);
        assert_eq!(tok.decode_bytes(&encoded).unwrap(), vec![b]);
    }
}

#[test]
fn property_paths_do_not_confuse_apis() {
    // decode_text on byte tokens whose concatenation is invalid UTF-8 must fail cleanly.
    let tok = Tokenizer::new_base();
    let raw = [0x80u8, 0x81, 0x82];
    let encoded = tok.encode_bytes(&raw);
    assert!(tok.decode_text(&encoded).is_err());
    assert_eq!(tok.decode_bytes(&encoded).unwrap(), raw);
}
