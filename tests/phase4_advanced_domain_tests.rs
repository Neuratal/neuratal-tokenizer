//! Phase 4 Mandatory Automated Tests: Advanced Domain Coverage
//!
//! Tests:
//! 1. Comprehensive Unicode handling (Combining marks, variation selectors, ZWJ/ZWNJ, mixed scripts)
//! 2. Advanced emoji test coverage (Skin tones, family ZWJ, flags, keycaps)
//! 3. Number handling (Decimals, negative, scientific notation, long integers, dates, hex, binary)
//! 4. Mathematical notation, LaTeX, and scientific symbols
//! 5. Programming-language coverage (Python, C, C++, Rust, JS, TS, Go, SQL, JSON)
//! 6. Mixed-language text and source code
//! 7. Trained tokenizer exact round-trip across all advanced domains

use neuratal_tokenizer::{
    CorpusManifest, CorpusManifestEntry, Normalization, PretokenizationMode, Tokenizer,
    TrainingConfig, train_from_manifest,
};
use std::fs::File;
use std::io::Write;
use std::sync::atomic::{AtomicUsize, Ordering};

static TEST_COUNTER: AtomicUsize = AtomicUsize::new(100);

#[test]
fn test_1_comprehensive_unicode_and_scripts() {
    let tok = Tokenizer::new_base();

    let unicode_cases = vec![
        // Combining diacritical marks in Latin
        "e\u{0301}a\u{0300}u\u{0308}c\u{0327}", // é, à, ü, ç via combining marks
        // Vietnamese stacked accents
        "tiếng Việt: đường lúa trĩu hạt, phở bò tái nạm, người ở lại",
        // Turkish dotted / dotless I
        "İstanbul - İSTANBUL - ıspanak - ISPANAK - ılık - İLİK",
        // Arabic harakat, tanween, and shaddah
        "الْعَرَبِيَّةُ: كَتَبَ، يَكْتُبُ، كِتَابَةً، كَاتِبٌ، مَكْتُوبٌ",
        // Hebrew with full niqqud
        "בְּרֵאשִׁית בָּרָא אֱלֹהִים אֵת הַשָּׁמַיִם וְאֵת הָאָרֶץ",
        // Devanagari conjuncts and halant
        "संस्कृतम्: विद्या ददाति विनयं विनयाद्याति पात्रताम्। क्ष, त्र, ज्ञ, श्र",
        // Thai vowels and tone marks (non-spaced)
        "ข้อความภาษาไทยที่มีวรรณยุกต์และสระซ้อน: น้ำแข็งไสอร่อยมาก",
        // Greek polytonic
        "Ἐν ἀρχῇ ἦν ὁ λόγος, καὶ ὁ λόγος ἦν πρὸς τὸν θεόν",
        // Zero-Width Joiner (ZWJ) & Zero-Width Non-Joiner (ZWNJ)
        "می‌خواهم (Persian ZWNJ \u{200C}) and Sinhala ZWJ ක\u{200D}ෂ",
        // Mixed scripts in a single sentence
        "English, Türkçe, Русский, العربية, עברית, 中文, 日本語, 한국어, हिन्दी, ไทย, Tiếng Việt!",
    ];

    for sample in unicode_cases {
        let encoded = tok.encode_text(sample).expect("Unicode encode failed");
        let decoded = tok.decode_text(&encoded).expect("Unicode decode failed");
        assert_eq!(
            decoded, sample,
            "Exact Unicode round-trip failed for sample: {}",
            sample
        );
    }
}

#[test]
fn test_2_advanced_emoji_coverage() {
    let tok = Tokenizer::new_base();

    let emoji_cases = vec![
        // Basic single codepoints
        "😀 😃 😄 😁 😆 😅 😂 🤣 🥲 ☺️ 😊 😇",
        // Fitzpatrick skin-tone modifiers (types 1-2, 3, 4, 5, 6)
        "👋🏻 👋🏼 👋🏽 👋🏾 👋🏿 👍🏻 👍🏼 👍🏽 👍🏾 👍🏿",
        // Complex gender and profession ZWJ sequences
        "👨‍💻 👩‍💻 👨‍🚒 👩‍🚒 👨‍🚀 👩‍🚀 👨‍⚕️ 👩‍⚕️ 👨‍🍳 👩‍🍳",
        // Complex family ZWJ sequences
        "👨‍👩‍👧 👨‍👩‍👧‍👦 👨‍👩‍👦‍👦 👨‍👩‍👧‍👧 👩‍👦 👩‍👧 👩‍👧‍👦 👨‍👦 👨‍👧 👨‍👧‍👦",
        // Regional indicator flags
        "🇹🇷 🇺🇸 🇬🇧 🇩🇪 🇫🇷 🇯🇵 🇨🇳 🇮🇳 🇧🇷 🇷🇺",
        // Multi-component flags (Rainbow, Transgender)
        "🏳️‍🌈 🏳️‍⚧️ 🏴‍☠️",
        // Keycap digit sequences with variation selectors and enclosing marks
        "0️⃣ 1️⃣ 2️⃣ 3️⃣ 4️⃣ 5️⃣ 6️⃣ 7️⃣ 8️⃣ 9️⃣ 🔟 *️⃣ #️⃣",
        // Mixed text, punctuation, numbers, and emojis
        "Order #1️⃣2️⃣3️⃣ shipped! 🚀 Total: $99.99 💸. Thank you 👍🏽! See you in İstanbul 🇹🇷✨",
    ];

    for sample in emoji_cases {
        let encoded = tok.encode_text(sample).expect("Emoji encode failed");
        let decoded = tok.decode_text(&encoded).expect("Emoji decode failed");
        assert_eq!(decoded, sample, "Emoji round-trip failed: {}", sample);
    }
}

#[test]
fn test_3_number_handling_coverage() {
    let tok = Tokenizer::new_base();

    let number_cases = vec![
        // Integers and signs
        "0 1 42 -1 -42 +100 +0 -0",
        // Long integer sequences
        "1234567890123456789012345678909876543210",
        // Decimals with dots and commas
        "3.141592653589793 -0.00001234 1,234,567.89 1.234.567,89",
        // Scientific notation
        "1e10 1e-10 -1.25e+20 6.02214076e23 2.99792458e8 1.602176634e-19",
        // Hexadecimal and Binary
        "0x0 0xFF 0x1A2B3C 0xdeadbeef 0xDEADBEEF 0b101010 0b11110000",
        // Dates, Times, and IP addresses
        "2026-11-14 2026/11/14 14.11.2026 23:59:59.999 192.168.1.1 255.255.255.0",
        // Currencies, Percentages, and Math powers
        "$100 €50 £75 ₺1.500,00 ¥10000 99.99% -0.5% 10^100 2^{16} 1/256",
    ];

    for sample in number_cases {
        let encoded = tok.encode_text(sample).expect("Number encode failed");
        let decoded = tok.decode_text(&encoded).expect("Number decode failed");
        assert_eq!(decoded, sample, "Number round-trip failed: {}", sample);
    }
}

#[test]
fn test_4_mathematics_and_scientific_notation() {
    let tok = Tokenizer::new_base();

    let math_cases = vec![
        // Standard arithmetic and algebra
        "a^2 + b^2 = c^2, \t (x + y)^n = \\sum_{k=0}^n \\binom{n}{k} x^{n-k} y^k",
        // Calculus: integrals, limits, partial derivatives
        "\\int_{-\\infty}^\\infty e^{-x^2} dx = \\sqrt{\\pi}, \t \\lim_{x \\to 0} \\frac{\\sin x}{x} = 1",
        "\\frac{\\partial f}{\\partial x} + \\frac{\\partial f}{\\partial y} = 0, \t \\nabla \\times \\mathbf{B} = \\mu_0 \\mathbf{J}",
        // Logic and set notation
        "\\forall x \\in \\mathbb{R}, \\exists y \\in \\mathbb{R}: x < y \\land y \\in (x, x+1)",
        "A \\cap B \\subseteq A \\cup B, \t x \\notin \\emptyset, \t P \\implies Q \\iff \\neg P \\lor Q",
        // Greek letters and scientific constants
        "α β γ δ ε ζ η θ ι κ λ μ ν ξ ο π ρ σ τ υ φ χ ψ ω",
        "Α Β Γ Δ Ε Ζ Η Θ Ι Κ Λ Μ Ν Ξ Ο Π Ρ Σ Τ Υ Φ Χ Ψ Ω",
        "c = 299792458 m/s, \t \\hbar = 1.054571817 \\times 10^{-34} J \\cdot s",
    ];

    for sample in math_cases {
        let encoded = tok.encode_text(sample).expect("Math encode failed");
        let decoded = tok.decode_text(&encoded).expect("Math decode failed");
        assert_eq!(decoded, sample, "Math round-trip failed: {}", sample);
    }
}

#[test]
fn test_5_programming_languages_coverage() {
    let tok = Tokenizer::new_base();

    // 1. Python
    let python = "def train_epoch(model, dataloader, optimizer):\n    model.train()\n    for batch in dataloader:\n        loss = model(batch)\n        loss.backward()\n        optimizer.step()\n";
    // 2. C / C++
    let cpp = "#include <iostream>\n#include <vector>\ntemplate<typename T>\nauto sum(const std::vector<T>& v) -> T {\n    T total = 0;\n    for (const auto& x : v) total += x;\n    return total;\n}\n";
    // 3. Rust
    let rust = "pub fn process_tokens<'a>(tokens: &'a [u32]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {\n    let mut buf = Vec::with_capacity(tokens.len());\n    tokens.iter().for_each(|&t| buf.push((t & 0xFF) as u8));\n    Ok(buf)\n}\n";
    // 4. JavaScript / TypeScript
    let ts = "interface User {\n  id: number;\n  name: string;\n  isActive: boolean;\n}\nconst filterUsers = (users: User[]): User[] => users.filter(u => u.isActive && u.id > 0);\n";
    // 5. Go
    let go = "package main\nimport \"fmt\"\nfunc worker(id int, jobs <-chan int, results chan<- int) {\n    for j := range jobs {\n        results <- j * 2\n    }\n}\n";
    // 6. SQL
    let sql = "WITH HighValueOrders AS (\n    SELECT customer_id, SUM(amount) AS total_spent\n    FROM orders\n    WHERE status = 'COMPLETED'\n    GROUP BY customer_id\n)\nSELECT c.name, h.total_spent FROM customers c JOIN HighValueOrders h ON c.id = h.customer_id;\n";
    // 7. JSON
    let json = "{\n  \"model\": \"Neuratal Tokenizer\",\n  \"version\": 1,\n  \"vocab_size\": 32768,\n  \"supported_languages\": [\"en\", \"tr\", \"ru\", \"zh\", \"ar\"],\n  \"is_lossless\": true\n}\n";

    let code_samples = vec![python, cpp, rust, ts, go, sql, json];

    for code in code_samples {
        let encoded = tok.encode_text(code).expect("Code encode failed");
        let decoded = tok.decode_text(&encoded).expect("Code decode failed");
        assert_eq!(
            decoded, code,
            "Source code round-trip failed! Indentation/tabs/operators must be preserved exactly."
        );
    }
}

#[test]
fn test_6_trained_model_exact_roundtrip_on_all_domains() {
    let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let temp_dir = std::env::temp_dir();
    let corpus_path = temp_dir.join(format!(
        "phase4_train_corpus_{}_{}.txt",
        std::process::id(),
        id
    ));

    {
        let mut f = File::create(&corpus_path).unwrap();
        // Populate corpus with multi-domain content
        for _ in 0..10 {
            writeln!(f, "def train_model(epochs=10):\n    for e in range(epochs):\n        print(f'Epoch {{e}}')").unwrap();
            writeln!(
                f,
                "Yapay zeka ve doğal dil işleme sistemleri çok dilli veri kümeleriyle eğitilir."
            )
            .unwrap();
            writeln!(f, "Уравнение теплопроводности: \\frac{{\\partial u}}{{\\partial t}} = \\alpha \\nabla^2 u").unwrap();
            writeln!(f, "النص العربي في الذكاء الاصطناعي مع التشكيل والرموز.").unwrap();
            writeln!(f, "自回归语言模型处理复杂的中文分词与句法分析。").unwrap();
            writeln!(
                f,
                "SELECT count(*), max(score) FROM results WHERE valid = true;"
            )
            .unwrap();
            writeln!(
                f,
                "Numbers: 123456789, 3.14159265, -0.05e-10, 0xABCDEF, 2026-11-14"
            )
            .unwrap();
            writeln!(f, "Emojis: 👨‍👩‍👧‍👦 🚀 🇹🇷 1️⃣ 👍🏽 ❤️ ✨").unwrap();
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
        target_vocab_size: 380,
        min_frequency: 2,
        pretokenizer: PretokenizationMode::ScriptAndDigitAware,
        normalization: Normalization::None,
        special_tokens: vec!["<|bos|>".to_string(), "<|eos|>".to_string()],
        seed: 42,
    };

    let (trained_tok, stats) = train_from_manifest(&manifest, &config).expect("Training failed");
    assert_eq!(stats.final_vocab_size, 380);

    // Verify exact roundtrip across all domains on the TRAINED tokenizer
    let verification_inputs = vec![
        "English text with contractions and technical terms: aren't, don't, multi-GPU training.",
        "Türkçe: Pijamalı hasta yağız şoföre çabucak güvendi. İstanbul, Iğdır, Şırnak.",
        "Русский: Съешь же ещё этих мягких французских булок, да выпей чаю.",
        "العربية: الرَّحْمَٰنِ الرَّحِيمِ - مَالِكِ يَوْمِ الدِّينِ",
        "中文：天地玄黄，宇宙洪荒。深度学习分词算法。",
        "def fib(n: int) -> int:\n    return n if n <= 1 else fib(n-1) + fib(n-2)",
        "\\oint_{\\partial \\Sigma} \\mathbf{A} \\cdot d\\mathbf{r} = \\iint_\\Sigma (\\nabla \\times \\mathbf{A}) \\cdot d\\mathbf{S}",
        "Numbers: -987.654e-32, 0x12345678, 2026-11-14T11:15:00Z",
        "Complex emojis: 👨‍👩‍👧‍👦 👩🏽‍💻 🏳️‍🌈 🇹🇷 1️⃣ 👍🏽",
    ];

    for input in verification_inputs {
        let encoded = trained_tok.encode_text(input).unwrap();
        let decoded = trained_tok.decode_text(&encoded).unwrap();
        assert_eq!(
            decoded, input,
            "Trained tokenizer failed roundtrip on: {}",
            input
        );
    }

    let _ = std::fs::remove_file(corpus_path);
}
