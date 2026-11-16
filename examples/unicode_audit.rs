//! Adversarial Unicode audit for NTokenizer (v0.3-v2 cycle).
//!
//! Checks, for every adversarial sample:
//!   1. pretokenizer slices tile the full input (concatenated slices == input)
//!   2. encode_text -> decode_text is byte-exact
//!   3. encode_bytes -> decode_bytes is byte-exact (works on invalid UTF-8 too)
//!
//! Usage: cargo run --release --example unicode_audit -- <model.ntok>

use neuratal_tokenizer::{PretokenizationMode, Tokenizer};

fn main() {
    let model = std::env::args().nth(1).expect("usage: unicode_audit <model.ntok>");
    let tok = Tokenizer::load_from_file(&model).expect("load model");

    let samples: Vec<(&str, &str)> = vec![
        ("turkish_dotted_I", "İstanbul Iğdır İyi günler TRİÇE"),
        ("turkish_decomposed", "I\u{0307}stanbul\u{0307} a\u{0307}i\u{0307} glu\u{0308}züm ş\u{0307}eker"),
        ("turkish_precomposed", "İğdır şişli güller ölçeği çim"),
        ("hindi_vowel_signs", "किताब की क़िला क्या कमी"),
        ("hindi_combining", "क\u{093F}त\u{093E}ब क\u{094D}ष\u{094D}य क\u{0943}षि"),
        ("hindi_nukta", "क़ ख़ ग़ ज़ ड़ ढ़ फ़"),
        ("devanagari_signs", "ा ि ी ु ू ृ े ै ो ौ ं ः ँ ्"),
        ("arabic_harakat", "بِسْمِ ٱللَّهِ ٱلرَّحْمَٰنِ ٱلرَّحِيمِ"),
        ("arabic_tamasuddiq", "مُحَمَّدٌ قُرْآنٌ"),
        ("emoji_zwj_family", "👨‍👩‍👧‍👦 🏳️‍🌈 👩🏽‍💻 🧑🏿‍🚀"),
        ("emoji_vs", "❤️ ✈︎ ✈️ ☀︎ ☀️"),
        ("emoji_skin_tone", "👍🏽 👋🏻 ✊🏿 🤏🏼"),
        ("flags_regional", "🇹🇷 🇺🇸 🇯🇵 🏴󠁧󠁢󠁥󠁮󠁧󠁿"),
        ("combining_diacritics", "e\u{0301}\u{0327} a\u{0300}\u{0316} o\u{0308}\u{0301} u\u{0301}\u{0308}"),
        ("enclosing_marks", "1\u{20DD} e\u{20E0} a\u{20E3} b\u{20E8}"),
        ("cjk", "中文，測試。日本語のテキスト。한국어 텍스트."),
        ("cjk_mixed", "Hello世界123，テスト。mixed 中文 with English 456。"),
        ("newlines_tabs", "a\nb\r\nc\td  \n\n\tef\r\n\r\n"),
        ("unusual_ws", "a\u{0085}b\u{00A0}c\u{2028}d\u{2029}e\u{202F}f\u{2007}g\u{3000}h"),
        ("bom", "\u{FEFF}text with BOM\u{FEFF} inside"),
        ("mixed_script", "αβγ中文δбгд"),
        ("rare_codepoints", "\u{10FFFF}\u{F0000}\u{0378}\u{E000}\u{FFFE}\u{FFFF}\u{10FFFD}"),
        ("math_symbols", "x² + y³ = z; ∑\u{2081}ⁿ a\u{2091} ≤ √(x²+y²) ≠ ∞; H₂O; 10²⁰"),
        ("greek_science", "λ ≈ 500 nm; μg/mL; ΔG° = −RT ln K; α\u{0308}-particles"),
        ("latex", r"\frac{a}{b} + \sqrt{x^{2}} \int_{0}^{1} f(t)\,dt"),
        ("thai", "ภาษาไทย สระ ི ึ ั ุ"),
        ("hebrew", "עִבְרִית שָׁלוֹם טוֹב"),
        ("prepend_mark_only", "\u{0301}\u{0300}\u{0327}"),
        ("leading_combining", "\u{0301}abc def\u{0301}"),
        ("leading_mark_newline", "\u{0301}\nline2"),
        ("leading_mark_cjk", "\u{0301}中文字"),
        ("mixed_leading_thai", "\u{0E34}\u{0E34}thai"),
        ("devanagari_leading", "\u{093F}किताब"),
        ("arabic_leading", "\u{064E}بِسْمِ"),
        ("zwj_only", "a\u{200D}b"),
        ("vs_only", "a\u{FE0F}b"),
        ("empty_and_space", " "),
        ("only_newlines", "\n\n\r\n"),
        ("soft_hyphen", "co\u{00AD}operate\u{AD}"),
        ("word_joiner", "a\u{2060}b\u{2060}c"),
        ("tags", "🏴\u{E0061}\u{E0062}\u{E0063}\u{E007F}"),
    ];

    let mut failures = 0usize;

    for mode in [
        PretokenizationMode::MathSymbolAware,
        PretokenizationMode::LeadingWhitespaceUnicode,
    ] {
        println!("=== pretokenizer {:?} ===", mode);
        for (name, text) in &samples {
            // 1. slices tile the input
            let slices = mode.split_text(text);
            let rebuilt: String = slices.concat();
            let tiles = rebuilt == *text;
            // 3. byte round trip
            let ids = neuratal_tokenizer::encoding::encode_bytes(
                text.as_bytes(),
                tok.merges(),
                &mode,
            );
            let back = neuratal_tokenizer::decoding::decode_bytes(
                &ids,
                tok.vocab(),
                tok.special_tokens(),
                &Default::default(),
            );
            let byte_ok = matches!(&back, Ok(b) if b == text.as_bytes());
            // 2. text round trip through the model's own pretokenizer
            let text_ids = tok.encode_text(text).unwrap_or_default();
            let text_back = tok.decode_text(&text_ids);
            let text_ok = matches!(&text_back, Ok(s) if s == *text);
            let ok = tiles && byte_ok && text_ok;
            if !ok {
                failures += 1;
                println!(
                    "FAIL {name}: tiles={tiles} byte_rt={byte_ok} text_rt={text_ok} slices={}",
                    slices.len()
                );
                if !tiles {
                    println!("   input  : {:?}", text);
                    println!("   rebuilt: {:?}", rebuilt);
                }
            } else {
                println!("ok   {name} ({} bytes, {} slices)", text.len(), slices.len());
            }
        }
    }

    // Invalid UTF-8 must survive encode_bytes/decode_bytes
    let invalid: Vec<u8> = vec![0xFF, 0x00, 0xFE, 0x80, 0xC0, 0xAF, 0xED, 0xA0, 0x80];
    let ids = neuratal_tokenizer::encoding::encode_bytes(
        &invalid,
        tok.merges(),
        &PretokenizationMode::MathSymbolAware,
    );
    let back = neuratal_tokenizer::decoding::decode_bytes(
        &ids,
        tok.vocab(),
        tok.special_tokens(),
        &Default::default(),
    )
    .unwrap();
    println!("=== invalid utf-8 round trip: {} ===", if back == invalid { "ok" } else { "FAIL" });
    if back != invalid {
        failures += 1;
    }

    println!("\nFAILURES: {failures}");
    std::process::exit(if failures == 0 { 0 } else { 1 });
}
