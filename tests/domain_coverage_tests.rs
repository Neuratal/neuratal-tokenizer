//! Domain coverage tests: multilingual/Unicode (incl. Bengali and rare
//! characters), mathematics/science, and programming languages.
//!
//! Every sample must round-trip exactly through both the base tokenizer and a
//! small trained model. Invalid UTF-8 is exercised through the binary API.
//!
//! These tests assert byte-exact reconstruction; they do not claim universal
//! Unicode coverage beyond the enumerated samples.

use neuratal_tokenizer::{
    CorpusManifest, CorpusManifestEntry, Normalization, PretokenizationMode, Tokenizer,
    TrainingConfig, train_from_manifest,
};
use std::sync::OnceLock;

fn trains_model() -> &'static Tokenizer {
    static MODEL: OnceLock<Tokenizer> = OnceLock::new();
    MODEL.get_or_init(|| {
        let corpus_path =
            std::env::temp_dir().join(format!("domain_corpus_{}.txt", std::process::id()));
        {
            use std::io::Write;
            let mut f = std::fs::File::create(&corpus_path).unwrap();
            for _ in 0..6 {
                writeln!(f, "the quick brown fox jumps over the lazy dog 0123456789").unwrap();
                writeln!(
                    f,
                    "Türkçe metin: Şemsi Paşa Pasajı, İstanbul, Iğdır, Çankırı, Şırnak"
                )
                .unwrap();
                writeln!(
                    f,
                    "Русский текст: машинное обучение и нейронные сети в 2026 году"
                )
                .unwrap();
                writeln!(
                    f,
                    "النص العربي: معالجة اللغة الطبيعية والتعلم الآلي والتشكيل"
                )
                .unwrap();
                writeln!(f, "中文文本：深度学习与自然语言处理模型和多语言语料库").unwrap();
                writeln!(
                    f,
                    "def compute(x):\n    return x * 2  # python sample with indentation"
                )
                .unwrap();
                writeln!(
                    f,
                    "SELECT id, COUNT(*) FROM events WHERE ts > '2026-01-01' GROUP BY id;"
                )
                .unwrap();
                writeln!(f, "∑∫√ π ∞ Δ λ μ α β γ E = mc^2").unwrap();
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
            target_vocab_size: 340,
            min_frequency: 2,
            pretokenizer: PretokenizationMode::ConservativeUnicode,
            normalization: Normalization::None,
            special_tokens: vec!["<|bos|>".to_string(), "<|eos|>".to_string()],
            seed: 42,
        };
        let (tok, _) = train_from_manifest(&manifest, &config).unwrap();
        let _ = std::fs::remove_file(corpus_path);
        tok
    })
}

/// Asserts exact round-trip through both tokenizers.
fn assert_roundtrip(sample: &str) {
    let base = Tokenizer::new_base();

    let ids = base.encode_text(sample).expect("base encode failed");
    let decoded = base.decode_text(&ids).expect("base decode failed");
    assert_eq!(decoded, sample, "base tokenizer round-trip failed");

    let trained = trains_model();
    let ids = trained.encode_text(sample).expect("trained encode failed");
    let decoded = trained.decode_text(&ids).expect("trained decode failed");
    assert_eq!(decoded, sample, "trained tokenizer round-trip failed");
}

#[test]
fn multilingual_major_scripts() {
    let samples = [
        // English
        "The quick brown fox jumps over the lazy dog. Pack my box with five dozen liquor jugs.",
        // Turkish (dotted İ, dotless ı, soft g, cedilla, circumflex)
        "İstanbul, Iğdır, Çankırı, Şırnak — Şemsi Paşa Pasajı'nda sesi boğuk bir şoför gördüm: ılık, ışık, oğul, gül",
        // German
        "Fußball-Europameisterschaft: Donaudampfschifffahrtselektrizitätenhauptbetriebswerkbauunterbeamtengesellschaft",
        // French
        "Où sont les neiges d'antan? Élévation, forêt, cœur, œuvre, garçon — très bien, très vite, ça va.",
        // Spanish
        "¿Cómo estás? ¡Excelente! Niño, señor, mañana, perro, corazón, Valparaíso, señorío.",
        // Russian (ё included)
        "Ёлку съели степные горные барсуки. Съешь ещё этих мягких французских булок, да выпей чаю.",
        // Greek (polytonic)
        "Ἐν ἀρχῇ ἦν ὁ λόγος, καὶ ὁ λόγος ἦν πρὸς τὸν θεόν, καὶ θεὸς ἦν ὁ λόγος.",
        // Arabic (with harakat)
        "نَصُّ الْعَرَبِيَّةِ: كَتَبَ الطَّالِبُ الدَّرْسَ فِي الْمَدْرَسَةِ. الْبُحُوثُ الْعِلْمِيَّةُ مُهِمَّةٌ.",
        // Hebrew (with niqqud)
        "בְּרֵאשִׁית בָּרָא אֱלֹהִים אֵת הַשָּׁמַיִם וְאֵת הָאָרֶץ. לָשׁוֹן הַקֹּדֶשׁ.",
        // Hindi (Devanagari conjuncts)
        "आधुनिक प्राकृतिक भाषा प्रसंस्करण प्रणालियां: क्ष त्र ज्ञ श्र संयुक्त अक्षर और मात्राएं",
        // Bengali
        "আধুনিক প্রাকৃতিক ভাষা প্রক্রিয়াকরণ ব্যবস্থা: বাংলা ভাষা এবং যৌগিক অক্ষর",
        // Thai (tone marks, non-spaced)
        "แบบจำลองภาษาขนาดใหญ่สามารถประมวลผลข้อความภาษาไทยที่ไม่มีการเว้นวรรคระหว่างคำได้",
        // Vietnamese (stacked accents)
        "Tiếng Việt: đường lúa trĩu hạt, phở bò tái nạm, nước mắt, nghĩa là gì?",
        // Chinese (simplified + traditional)
        "深度学习自然语言处理模型。機器學習模型利用自動歸納的特征處理多語言語料庫。",
        // Japanese (kanji + hiragana + katakana)
        "漢字とひらがなとカタカナが混ざる。大規模言語モデルは日本語のトークナイザーを学習する。",
        // Korean (Hangul syllables + jamo)
        "한글 음절과 자모를 사용하는 한국어: ㅤㅏㅁᆞ 조합과 자음, 모음, 받침",
        // Mixed-script paragraph
        "Mixed: English, Türkçe, Русский, العربية, עברית, 中文, 日本語, 한국어, हिन्दी, ไทย, Tiếng Việt — all in one line! 12345",
    ];

    for sample in samples {
        assert_roundtrip(sample);
    }
}

#[test]
fn multilingual_rare_and_special_codepoints() {
    let samples = [
        // Mathematical alphanumeric symbols (plane 1)
        "𝐴 + 𝐵 = 𝐶,  𝑥² + 𝑦² = 𝑧²,  𝑓(𝑥) = ∫ₐᵇ 𝑔(𝑡) 𝑑𝑡,  𝒩(μ, σ²)",
        // CJK Extension B (rare hanzi, plane 2)
        "𠮷野家の𩸽\u{29E3D}は美味しい。",
        // Mongolian script
        "ᠮᠣᠩᠭᠣᠯ ᠬᠡᠯᠡ ᠪᠢᠴᠢᠭ",
        // Deseret (constructed-ish rare Latin)
        "𐐨𐑇 𐐨𐑀 𐐨𐑌𐐨𐑁𐐨𐑌𐐨𐐻",
        // RTL override + Arabic
        "\u{202E}العربية \u{202C} outside",
        // Tags block (language tag chars)
        "tagged: \u{E0021}\u{E0041}\u{E0042}\u{E007F}",
        // Mongolian vowel separator + Mongolian letters
        "a\u{180E}b\u{180F}",
        // Zero-width characters alone
        "zero\u{200B}width\u{200C}spaces\u{200D}joiners",
    ];

    for sample in samples {
        assert_roundtrip(sample);
    }
}

#[test]
fn combining_marks_variation_selectors_zwj() {
    let samples = [
        // Stacked combining marks (Latin): e + acute + dot below + circumflex
        "e\u{0301}\u{0323}\u{0302}",
        // Vietnamese stacked accents
        "Ti\u{1EBF}ng Vi\u{1EC7}t c\u{1EE7}a ch\u{00FA}ng t\u{00F4}i",
        // Hebrew with niqqud
        "\u{05D1}\u{05B0}\u{05E8}\u{05B6}\u{05D0}\u{05E9}\u{05B4}\u{05D9}\u{05EA}",
        // Arabic with tatweel + shadda + fatha
        "كـَــتَ\u{0651}ـبَ",
        // Devanagari conjunct with halant
        "\u{0915}\u{094D}\u{0937}\u{0915}\u{094D}\u{0930}\u{091C}\u{094D}\u{091E}",
        // Thai tone marks
        "\u{0E01}\u{0E35}\u{0E48}\u{0E19}",
        // Variation selectors: heart text vs emoji vs emoji + VS16
        "♥ ♥️ \u{2764}\u{FE0F}",
        "✌ ✌\u{FE0F} ☺ ☺\u{FE0F} ☀ ☀\u{FE0E} ☀\u{FE0F}",
        // Family ZWJ sequence
        "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}\u{200D}\u{1F466}",
        // Woman technologist + medium skin tone
        "\u{1F469}\u{1F3FD}\u{200D}\u{1F4BB}",
        // Flags: regional indicator sequences
        "\u{1F1F9}\u{1F1F7} \u{1F1FA}\u{1F1F8} \u{1F1E9}\u{1F1EA} \u{1F3F4}\u{E0067}\u{E0062}\u{E006E}\u{E0067}\u{E007F}",
        // Skin tones
        "\u{1F44D}\u{1F3FB} \u{1F44D}\u{1F3FC} \u{1F44D}\u{1F3FD} \u{1F44D}\u{1F3FE} \u{1F44D}\u{1F3FF}",
        // Keycap sequences
        "1\u{FE0F}\u{20E3} 2\u{FE0F}\u{20E3} #\u{FE0F}\u{20E3} *\u{FE0F}\u{20E3}",
        // Mixed text with ZWJ between letters (ZWNJ in Persian)
        "می\u{200C}خواهم",
        // Emoji modifier with ZWJ
        "\u{1F3FB}\u{1F3FB}\u{1F3FF}",
    ];

    for sample in samples {
        assert_roundtrip(sample);
    }
}

#[test]
fn math_and_science() {
    let samples = [
        // Integers, decimals, negatives
        "123 -987 3.141592653589793 0 1000000 -0.001 -273.15",
        // Scientific notation
        "6.02214076e+23 -1.602e-19 1.25e+10 2.998e8 10^100 \u{00D7}",
        // Fractions
        "\u{00BD} \u{00BC} \u{00BE} \u{2153} 3/4 22/7",
        // Exponents and superscripts
        "x^10 \u{2075} 10^{-3} e^{i\u{03C0}} + 1 = 0 2\u{207B}\u{00B9}",
        // Equations
        "E = mc^2  F = ma  a^2 + b^2 = c^2  pV = nRT  A = \u{03C0}r^2",
        // Greek mathematical symbols
        "\u{03B1} \u{03B2} \u{03B3} \u{03BB} \u{03BC} \u{03C0} \u{03C3} \u{03C6} \u{03C9} \u{0394} \u{039B} \u{03A3} \u{03A9}",
        // Sums, integrals, limits
        "\u{2211}_{i=1}^{n} i = n(n+1)/2  \u{222B}_{0}^{1} x^2 dx = 1/3  \u{2202}f/\u{2202}x  \u{2207}\u{00B7}F = \u{03C1}",
        // LaTeX
        "\\frac{1}{\u{221A}2} \\int_{-\u{221E}}^{\u{221E}} e^{-x^2} dx = \u{221A}\u{03C0} \\lim_{x \u{2192} 0} \u{2202}",
        // Chemical formulas
        "H\u{2082}O CO\u{2082} CH\u{2084} H\u{2082}SO\u{2084} C\u{2088}H\u{2081}\u{2080}N\u{2084}O\u{2082} CH\u{2083}CH\u{2082}OH NaCl C\u{2086}H\u{2081}\u{2082}O\u{2086}",
        // Physical constants and units
        "h = 6.62607015 \u{00D7} 10^{-34} J\u{00B7}s  c = 299,792,458 m/s  R = 8.314 J/(mol\u{00B7}K)",
        // Derived units and notation
        "kg\u{00B7}m/s m/s^2 N, J, W, Pa, Hz, eV, mol, cd",
        // Pipe table (markdown-like)
        "| Symbol | Meaning | Value |\n|--------|---------|-------|\n| c | speed of light | 299792458 m/s |\n| g | gravity | 9.80665 m/s^2 |",
        // Set theory and logic
        "\u{2200}x \u{2208} A: p(x)  \u{2203}y \u{2208} B  \u{00AC}p \u{2227} q \u{2228} r  A \u{2286} B",
        // Matrices in LaTeX
        "\\begin{bmatrix} a & b \\\\ c & d \\end{bmatrix} \\det(A) = ad - bc",
        // Chemistry reactions
        "2H\u{2082} + O\u{2082} \u{2192} 2H\u{2082}O",
    ];

    for sample in samples {
        assert_roundtrip(sample);
    }
}

#[test]
fn programming_languages() {
    let samples = [
        // Python
        "def f(x: int) -> list[int]:\n    '''docstring'''\n    return [i * i for i in range(x) if i % 2 == 0]  # comment\n",
        // C
        "#include <stdio.h>\nint main(void) {\n    const char *s = \"quotes: \\\"escape\\\" and \\n\";\n    printf(\"%s\\n\", s);\n    return 0;\n}\n",
        // C++
        "#include <vector>\n#include <string>\nstd::vector<std::string> v{\"a\", \"b\"};\ntemplate <typename T> T add(T a, T b) { return a + b; }\n",
        // Rust
        "fn main() -> Result<(), Box<dyn std::error::Error>> {\n    let raw = br#\"raw string \"with quotes\" and \\n\"#;\n    println!(\"{:?}\", raw);\n    Ok(())\n}\n",
        // JavaScript
        "const f = async (x) => {\n  const tpl = `template ${x + 1} \\n multiline`;\n  return /regex\\d+/g.test(tpl) && tpl.includes(\"x\");\n};\n",
        // TypeScript
        "type Result<T, E = string> = { ok: true; value: T } | { ok: false; error: E };\ninterface User { id: number; name: string; tags?: readonly string[] }\n",
        // Java
        "public final class Main {\n    public static void main(String[] args) {\n        var list = java.util.List.of(\"a\", \"b\");\n        list.forEach(System.out::println);\n    }\n}\n",
        // Go
        "package main\n\nimport \"fmt\"\n\nfunc main() {\n\tname := \"gopher\"\n\tmsg := fmt.Sprintf(\"hello %s\\n\", name)\n\tfmt.Print(msg)\n}\n",
        // SQL
        "SELECT u.id, COUNT(o.id) AS orders\nFROM users u\nLEFT JOIN orders o ON o.user_id = u.id\nWHERE o.created_at >= DATE '2026-01-01'\nGROUP BY u.id\nHAVING COUNT(o.id) > 10;\n",
        // HTML
        "<!DOCTYPE html>\n<html lang=\"en\">\n<head><meta charset=\"utf-8\"><title>Test</title></head>\n<body>\n  <p class=\"main\">Hello &amp; welcome &mdash; <a href=\"https://example.com?a=1&b=2\">link</a></p>\n</body>\n</html>\n",
        // CSS
        "body { margin: 0; font-family: system-ui, sans-serif; }\n.btn:hover { background: #1a2b3c; transform: translateY(-1px); }\n@media (min-width: 768px) { .grid { grid-template-columns: repeat(3, 1fr); } }\n",
        // JSON
        "{\"name\": \"Neuratal Tokenizer\", \"version\": \"0.2.0\", \"tags\": [\"bpe\", \"unicode\"], \"meta\": {\"unicode\": true, \"count\": 256}}\n",
        // YAML
        "version: 1\nmodel:\n  architecture: transformer\n  layers: 10\n  hidden: 512\nsampling:\n  temperature: 0.7\n  top_p: 0.9\n  sequences:\n    - alpha\n    - beta\n",
        // Shell
        "#!/bin/bash\nset -euo pipefail\nfor f in *.txt; do\n  if [ -s \"$f\" ]; then echo \"processing $f\"; fi\ndone\nfind . -name '*.rs' -exec grep -l 'unsafe' {} \\;\n",
        // Indentation + tabs preservation
        "level0\n  level1 (2 spaces)\n    level2 (4 spaces)\n\tlevel1 (tab)\n\t\tlevel2 (2 tabs)\n",
        // Quoting and escaping soup
        "\"double quotes\" and 'single quotes' and \\\"escaped doubles\\\" and \\\"nested \\'inside\\' \\\"\n",
        // Regex soup
        "re: ^[a-zA-Z_][a-zA-Z0-9_]*$  pattern: \\\\bword\\\\b  flags: gimsu\n",
        // Comment styles across languages
        "// C++ comment\n/* C block\n   multi-line */\n# hash comment\n-- SQL comment\n{- Haskell -}\n<!-- HTML comment -->\n",
        // Backslash chains
        "path: C:\\\\Users\\\\example\\\\file.txt  regex: \\\\w+\\\\s+  escape: \\\\n\\\\t\\\\r\\\\0\n",
    ];

    for sample in samples {
        assert_roundtrip(sample);
    }
}

#[test]
fn invalid_utf8_and_binary_bytes_roundtrip() {
    let base = Tokenizer::new_base();
    let trained = trains_model();

    for tok in [&base, trained] {
        let raw: &[u8] = &[
            0x00, 0xFF, 0xFE, 0x80, 0x00, 0x7F, 0xC0, 0xC1, 0xF5, 0xF8, b'a', b'\n', 0xE2, 0x82,
            0xAC,
        ];
        let ids = tok.encode_bytes(raw);
        let decoded = tok.decode_bytes(&ids).expect("decode_bytes failed");
        assert_eq!(decoded, raw, "binary round-trip failed");
    }
}

#[test]
fn exact_bytes_preserved_without_normalization() {
    let base = Tokenizer::new_base();
    // NFD vs NFC forms of the same text must NOT be folded together
    let nfc = "é";
    let nfd = "e\u{0301}";
    assert_ne!(nfc.as_bytes(), nfd.as_bytes());

    for text in [nfc, nfd, "İ\u{0307}", "a\u{0301}b\u{0301}c"] {
        let ids = base.encode_text(text).unwrap();
        assert_eq!(base.decode_text(&ids).unwrap(), text);
    }
}
