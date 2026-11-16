//! Phase 2 Mandatory Automated Tests
//!
//! Tests the foundational components of Neuratal Tokenizer:
//! 1. All 256 byte values
//! 2. Empty input
//! 3. One-byte input
//! 4. Long input
//! 5. Invalid UTF-8 through the binary API
//! 6. Multilingual UTF-8 (14+ scripts and languages)
//! 7. Complex emoji sequences (ZWJ, modifiers, flags, keycaps)
//! 8. Mathematical symbols and notation
//! 9. Source code with whitespace, tabs, and indentation
//! 10. Exact round-trip preservation
//! 11. Special-token ID stability and collision prevention

use neuratal_tokenizer::{
    AllowedSpecial, DecodeOptions, EncodeOptions, PretokenizationMode, Tokenizer, TokenizerError,
};

#[test]
fn test_1_all_256_byte_values() {
    let tok = Tokenizer::new_base();
    let all_bytes: Vec<u8> = (0u8..=255u8).collect();

    // Test through binary API
    let token_ids = tok.encode_bytes(&all_bytes);
    assert_eq!(token_ids.len(), 256);
    for (i, &id) in token_ids.iter().enumerate() {
        assert_eq!(id, i as u32, "Byte {} must map to token ID {}", i, i);
    }

    let decoded = tok
        .decode_bytes(&token_ids)
        .expect("Binary decoding failed");
    assert_eq!(decoded, all_bytes, "All 256 bytes must round-trip exactly");
}

#[test]
fn test_2_empty_input() {
    let tok = Tokenizer::new_base();

    // Text API
    let encoded_text = tok.encode_text("").expect("Empty string encode failed");
    assert!(encoded_text.is_empty());
    let decoded_text = tok
        .decode_text(&encoded_text)
        .expect("Empty string decode failed");
    assert_eq!(decoded_text, "");

    // Binary API
    let encoded_bytes = tok.encode_bytes(b"");
    assert!(encoded_bytes.is_empty());
    let decoded_bytes = tok
        .decode_bytes(&encoded_bytes)
        .expect("Empty bytes decode failed");
    assert!(decoded_bytes.is_empty());
}

#[test]
fn test_3_one_byte_input() {
    let tok = Tokenizer::new_base();

    for b in 0u8..=255u8 {
        let input = [b];
        let encoded = tok.encode_bytes(&input);
        assert_eq!(encoded, vec![b as u32]);
        let decoded = tok
            .decode_bytes(&encoded)
            .expect("Single byte decode failed");
        assert_eq!(decoded, vec![b]);
    }
}

#[test]
fn test_4_long_input() {
    let tok = Tokenizer::new_base();

    // Generate 100 KB of repetitive and varied text
    let mut long_text = String::with_capacity(100_000);
    for i in 0..2000 {
        long_text.push_str("The quick brown fox jumps over the lazy dog. ");
        long_text.push_str("Türkçe metin: Şemsi Paşa Pasajı. ");
        long_text.push_str(&format!("Count: {}. \n", i));
    }

    let encoded = tok
        .encode_text(&long_text)
        .expect("Long input encode failed");
    let decoded = tok.decode_text(&encoded).expect("Long input decode failed");
    assert_eq!(decoded, long_text);
}

#[test]
fn test_5_invalid_utf8_through_binary_api() {
    let tok = Tokenizer::new_base();

    // Invalid UTF-8 byte sequences:
    // - 0xFF, 0xFE (never valid in UTF-8)
    // - 0xC0, 0xAF (overlong encoding)
    // - 0xED, 0xA0, 0x80 (surrogate pair)
    // - Trailing truncated multi-byte sequence
    let invalid_sequences: Vec<&[u8]> = vec![
        &[0xFF],
        &[0xFE, 0xFF],
        &[0xC0, 0xAF],
        &[0xED, 0xA0, 0x80],
        &[b'a', b'b', 0x80, 0x81, b'c'],
        &[0xF0, 0x90, 0x80], // truncated 4-byte UTF-8
    ];

    for seq in invalid_sequences {
        // Must succeed without panic in binary API
        let encoded = tok.encode_bytes(seq);
        let decoded = tok.decode_bytes(&encoded).expect("Binary decoding failed");
        assert_eq!(
            decoded, seq,
            "Binary round-trip failed for invalid UTF-8 sequence"
        );

        // Must correctly fail in text decode API (no silent character replacement)
        let text_res = tok.decode_text(&encoded);
        assert!(
            text_res.is_err(),
            "decode_text must return error on invalid UTF-8 without silent replacement"
        );
        assert!(matches!(
            text_res.unwrap_err(),
            TokenizerError::InvalidUtf8(_)
        ));
    }
}

#[test]
fn test_6_multilingual_utf8() {
    let tok = Tokenizer::new_base();

    let multilingual_samples = vec![
        // English
        "The quick brown fox jumps over the lazy dog.",
        // Turkish (with dotted/dotless I, accents)
        "Pijamalı hasta yağız şoföre çabucak güvendi. İstanbul, Iğdır, Şırnak, Çankırı.",
        // Cyrillic (Russian, Ukrainian)
        "Съешь же ещё этих мягких французских булок, да выпей чаю. Привіт, світе!",
        // Arabic (with diacritics and RTL)
        "نص عربي تجريبي مع تشكيل: الرَّحْمَٰنِ الرَّحِيمِ.",
        // Hebrew (with niqqud)
        "שָׁלוֹם עֲלֵיכֶם מַלְאֲכֵי הַשָּׁלוֹם",
        // Greek
        "Ξεσκεπάζω την ψυχοφθόρα βδελυγμία. Ἐν ἀρχῇ ἦν ὁ λόγος.",
        // Chinese (Simplified & Traditional)
        "天地玄黄，宇宙洪荒。日月盈昃，辰宿列张。繁體中文測試。",
        // Japanese (Kanji, Hiragana, Katakana)
        "いろはにほへと ちりぬるを わかよたれそ つねならむ。カタカナと漢字。",
        // Korean (Hangul)
        "키스의 고유조건은 입술끼리 만나야 하고 특별한 기술은 필요치 않다.",
        // Hindi / Indic (Devanagari conjuncts)
        "नमस्ते दुनिया! कर्मण्येवाधिकारस्ते मा फलेषु कदाचन।",
        // Thai (no spaces between words)
        "ภาษาไทยไม่มีช่องว่างระหว่างคำ แต่สามารถประมวลผลได้อย่างถูกต้อง",
        // Vietnamese (complex tone marks)
        "Việt Nam đất nước ta ơi! Mênh mông biển lúa đâu trời đẹp hơn.",
    ];

    for sample in multilingual_samples {
        let encoded = tok.encode_text(sample).expect("Multilingual encode failed");
        let decoded = tok
            .decode_text(&encoded)
            .expect("Multilingual decode failed");
        assert_eq!(
            decoded, sample,
            "Multilingual sample failed exact round-trip: {}",
            sample
        );
    }
}

#[test]
fn test_7_complex_emoji_sequences() {
    let tok = Tokenizer::new_base();

    let emoji_samples = vec![
        "😀",
        "😂",
        "❤️",
        "👍🏽", // Skin-tone modifier
        "👨‍👩‍👧‍👦", // Family ZWJ sequence (Man + ZWJ + Woman + ZWJ + Girl + ZWJ + Boy)
        "👩🏽‍💻", // Woman + medium skin tone + ZWJ + Laptop
        "🏳️‍🌈", // Rainbow flag (White flag + VS16 + ZWJ + Rainbow)
        "🇹🇷", // Flag of Turkey (Regional indicator T + Regional indicator R)
        "1️⃣", // Keycap digit 1 (Digit 1 + VS16 + Combining enclosing keycap)
        "Emoji in text: Selam 👋🏽! Nasıl gidiyor? 🚀✨",
    ];

    for sample in emoji_samples {
        let encoded = tok.encode_text(sample).expect("Emoji encode failed");
        let decoded = tok.decode_text(&encoded).expect("Emoji decode failed");
        assert_eq!(
            decoded, sample,
            "Emoji sample failed exact roundtrip: {}",
            sample
        );
    }
}

#[test]
fn test_8_mathematical_symbols() {
    let tok = Tokenizer::new_base();

    let math_samples = vec![
        "∑_{i=1}^n i = \\frac{n(n+1)}{2}",
        "∫_0^\\infty e^{-x^2} dx = \\frac{\\sqrt{\\pi}}{2}",
        "√x + y ≠ z ≤ w ≥ v ± ε × δ ÷ θ ≈ 0",
        "∂f/∂x + ∇ \\cdot E = ρ / ε_0",
        "∀x ∈ ℝ, ∃y ∈ ℝ: x + y = 0",
        "A → B ↔ C, π ≈ 3.141592653589793, Ω = 50 kΩ",
    ];

    for sample in math_samples {
        let encoded = tok.encode_text(sample).expect("Math encode failed");
        let decoded = tok.decode_text(&encoded).expect("Math decode failed");
        assert_eq!(
            decoded, sample,
            "Math sample failed exact roundtrip: {}",
            sample
        );
    }
}

#[test]
fn test_9_source_code_indentation_and_tabs() {
    let tok = Tokenizer::new_base();

    let python_code = r#"def fibonacci(n: int) -> int:
    """Calculates n-th Fibonacci number."""
    if n <= 1:
        return n
    a, b = 0, 1
    for _ in range(n):
        a, b = b, a + b
    return a
"#;

    let rust_code = "\tpub fn add(a: u32, b: u32) -> u32 {\n\t\ta + b\n\t}\n";

    let html_code = "<div class=\"container\">\n    <p>Hello &amp; welcome!</p>\n</div>";

    for &code in &[python_code, rust_code, html_code] {
        let encoded = tok.encode_text(code).expect("Code encode failed");
        let decoded = tok.decode_text(&encoded).expect("Code decode failed");
        assert_eq!(
            decoded, code,
            "Source code must preserve exact whitespace and tabs"
        );
    }
}

#[test]
fn test_10_exact_roundtrip_with_merges() {
    let mut tok = Tokenizer::new_base();

    // Add learned merges
    // 1. 't' + 'h' -> 256
    let id_th = tok.add_merge(b't' as u32, b'h' as u32).unwrap();
    assert_eq!(id_th, 256);
    // 2. 256 + 'e' -> 257 ("the")
    let id_the = tok.add_merge(256, b'e' as u32).unwrap();
    assert_eq!(id_the, 257);
    // 3. ' ' + 257 -> 258 (" the")
    let id_space_the = tok.add_merge(b' ' as u32, 257).unwrap();
    assert_eq!(id_space_the, 258);

    // In RawByte mode, "the the" should use merge 257 and 258
    tok.set_pretokenizer(PretokenizationMode::RawByte);
    let text = "the the";
    let encoded = tok.encode_text(text).unwrap();
    assert_eq!(encoded, vec![257, 258]);

    let decoded = tok.decode_text(&encoded).unwrap();
    assert_eq!(decoded, text);
}

#[test]
fn test_11_special_token_id_stability_and_collision_prevention() {
    let mut tok = Tokenizer::new_base();

    // 1. Attempting to register in base byte range (0..=255) must fail
    assert!(tok.register_special_token("<|invalid|>", 0).is_err());
    assert!(tok.register_special_token("<|invalid|>", 255).is_err());

    // 2. Register valid special tokens
    tok.register_special_token("<|bos|>", 300).unwrap();
    tok.register_special_token("<|eos|>", 301).unwrap();

    // 3. Duplicate ID or token registration must fail
    assert!(tok.register_special_token("<|dup_id|>", 300).is_err());
    assert!(tok.register_special_token("<|bos|>", 302).is_err());

    // 4. Stable encoding with special tokens enabled
    let text = "<|bos|>System prompt here.<|eos|>";
    let encoded = tok.encode_text(text).unwrap();
    assert_eq!(encoded[0], 300);
    assert_eq!(*encoded.last().unwrap(), 301);

    let decoded = tok.decode_text(&encoded).unwrap();
    assert_eq!(decoded, text);

    // 5. Test with skip_special_tokens = true
    let skip_opts = DecodeOptions {
        skip_special_tokens: true,
    };
    let stripped = tok.decode_text_with_options(&encoded, &skip_opts).unwrap();
    assert_eq!(stripped, "System prompt here.");

    // 6. Test with AllowedSpecial::None (prevents prompt injection)
    let safe_encode_opts = EncodeOptions {
        allowed_special: AllowedSpecial::None,
        normalization: tok.normalization(),
        pretokenizer: tok.pretokenizer(),
    };
    let safe_encoded = tok
        .encode_text_with_options(text, &safe_encode_opts)
        .unwrap();
    // In safe mode, "<|bos|>" is NOT emitted as 300, but as individual text bytes
    assert_ne!(safe_encoded[0], 300);
    assert_eq!(tok.decode_text(&safe_encoded).unwrap(), text);
}
