//! Phase 3 Mandatory Automated Tests
//!
//! Tests the high-performance BPE training engine:
//! 1. Training completes successfully on a synthetic multilingual corpus.
//! 2. Vocabulary size is correct and matches configuration.
//! 3. All generated merge IDs are valid (>= 256, contiguous, within bounds).
//! 4. Encoding after training reduces token count on repetitive examples.
//! 5. decode(encode(text)) reproduces the original text exactly.
//! 6. Deterministic training: identical inputs & seeds produce identical vocabularies and merge ranks.
//! 7. All three pre-tokenization strategies train and encode successfully.
//! 8. Saving and loading a trained tokenizer preserves its exact behavior.
//! 9. Invalid corpus records are reported correctly without silent failure.

use neuratal_tokenizer::{
    CorpusManifest, CorpusManifestEntry, Normalization, PretokenizationMode, Tokenizer,
    TrainingConfig, load_ntok_file, save_ntok_file, train_from_manifest,
};
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

use std::sync::atomic::{AtomicUsize, Ordering};

static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);

/// Helper function to create a synthetic multilingual corpus with a unique path per test.
fn create_synthetic_corpus(test_name: &str) -> (PathBuf, PathBuf) {
    let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let temp_dir = std::env::temp_dir();
    let txt_path = temp_dir.join(format!(
        "synthetic_{}_{}_{}.txt",
        test_name,
        std::process::id(),
        id
    ));
    let jsonl_path = temp_dir.join(format!(
        "synthetic_{}_{}_{}.jsonl",
        test_name,
        std::process::id(),
        id
    ));

    // 1. Plain text file with multilingual lines
    {
        let mut f = File::create(&txt_path).unwrap();
        for _ in 0..15 {
            writeln!(
                f,
                "Machine learning algorithms learn representations from data."
            )
            .unwrap();
            writeln!(
                f,
                "Yapay zeka modelleri doğal dil işleme görevlerinde kullanılır."
            )
            .unwrap();
            writeln!(
                f,
                "Алгоритмы машинного обучения обрабатывают многоязычные данные."
            )
            .unwrap();
            writeln!(
                f,
                "تعتمد نماذج معالجة اللغة الطبيعية على خوارزميات التعلم الآلي."
            )
            .unwrap();
            writeln!(f, "自然语言处理系统通过多语言分词器实现高效表示。").unwrap();
            writeln!(
                f,
                "def compute_loss(y_true, y_pred):\n    return ((y_true - y_pred) ** 2).mean()"
            )
            .unwrap();
            writeln!(
                f,
                "E = mc^2, \\int_0^1 x^2 dx = 1/3, \\sum_{{i=1}}^n i = n(n+1)/2"
            )
            .unwrap();
        }
    }

    // 2. JSONL file with multilingual records + deliberate malformed records
    {
        let mut f = File::create(&jsonl_path).unwrap();
        for _ in 0..10 {
            writeln!(f, r#"{{"text": "Autoregressive Transformers predict next tokens iteratively.", "language": "en"}}"#).unwrap();
            writeln!(f, r#"{{"text": "Büyük dil modelleri çok dilli veri kümeleriyle eğitilir.", "language": "tr"}}"#).unwrap();
            writeln!(f, r#"{{"text": "Многоязычный токенизатор сохраняет порядок байтов.", "language": "ru"}}"#).unwrap();
            writeln!(
                f,
                r#"{{"text": "123456789 + 987654321 = 1111111110", "domain": "math"}}"#
            )
            .unwrap();
        }
        // Malformed records to verify error reporting
        writeln!(f, r#"{{"missing_text_key": 12345}}"#).unwrap();
        writeln!(f, r#"NOT_VALID_JSON_AT_ALL_LINE"#).unwrap();
        writeln!(f, r#"{{"text": 99999}}"#).unwrap(); // text is not a string
    }

    (txt_path, jsonl_path)
}

#[test]
fn test_1_training_completes_and_vocab_size_correct() {
    let (txt_path, jsonl_path) = create_synthetic_corpus("test_1");

    let mut manifest = CorpusManifest::new(42);
    manifest.add_entry(CorpusManifestEntry {
        path: txt_path.clone(),
        weight: 1.0,
        language: None,
        domain: None,
        text_field: None,
    });
    manifest.add_entry(CorpusManifestEntry {
        path: jsonl_path.clone(),
        weight: 1.0,
        language: None,
        domain: None,
        text_field: None,
    });

    let target_size = 320; // 256 base bytes + 60 merges + 4 special tokens
    let config = TrainingConfig {
        target_vocab_size: target_size,
        min_frequency: 2,
        pretokenizer: PretokenizationMode::ScriptAndDigitAware,
        normalization: Normalization::None,
        special_tokens: vec![
            "<|bos|>".to_string(),
            "<|eos|>".to_string(),
            "<|pad|>".to_string(),
            "<|unk|>".to_string(),
        ],
        seed: 42,
    };

    let (tok, stats) = train_from_manifest(&manifest, &config).expect("Training failed");

    assert_eq!(stats.final_vocab_size, target_size);
    assert_eq!(tok.vocab_size(), target_size);
    assert_eq!(stats.merges_learned, 60);

    let _ = std::fs::remove_file(txt_path);
    let _ = std::fs::remove_file(jsonl_path);
}

#[test]
fn test_2_all_generated_merge_ids_are_valid() {
    let (txt_path, jsonl_path) = create_synthetic_corpus("test_2");

    let mut manifest = CorpusManifest::new(42);
    manifest.add_entry(CorpusManifestEntry {
        path: txt_path.clone(),
        weight: 1.0,
        language: None,
        domain: None,
        text_field: None,
    });

    let config = TrainingConfig {
        target_vocab_size: 300, // 256 base + 40 merges + 4 special
        min_frequency: 2,
        pretokenizer: PretokenizationMode::ConservativeUnicode,
        normalization: Normalization::None,
        special_tokens: vec![
            "<|bos|>".to_string(),
            "<|eos|>".to_string(),
            "<|pad|>".to_string(),
            "<|unk|>".to_string(),
        ],
        seed: 42,
    };

    let (tok, _) = train_from_manifest(&manifest, &config).unwrap();

    let rules = tok.merges().rules();
    assert_eq!(rules.len(), 40);

    for (rank, rule) in rules.iter().enumerate() {
        assert_eq!(rule.rank, rank as u32);
        assert_eq!(rule.target, (256 + rank) as u32);
        assert!(rule.target >= 256);
        assert!(rule.target < tok.vocab_size() as u32);
        assert!(tok.vocab().get_bytes(rule.target).is_some());
    }

    let _ = std::fs::remove_file(txt_path);
    let _ = std::fs::remove_file(jsonl_path);
}

#[test]
fn test_3_compression_token_count_reduction() {
    let (txt_path, _) = create_synthetic_corpus("test_3");

    let mut manifest = CorpusManifest::new(42);
    manifest.add_entry(CorpusManifestEntry {
        path: txt_path.clone(),
        weight: 1.0,
        language: None,
        domain: None,
        text_field: None,
    });

    let config = TrainingConfig {
        target_vocab_size: 350,
        min_frequency: 2,
        pretokenizer: PretokenizationMode::ScriptAndDigitAware,
        normalization: Normalization::None,
        special_tokens: vec!["<|bos|>".to_string(), "<|eos|>".to_string()],
        seed: 42,
    };

    let (trained_tok, _) = train_from_manifest(&manifest, &config).unwrap();
    let base_tok = Tokenizer::new_base();

    let test_sentence = "Machine learning algorithms learn representations from data.";
    let base_tokens = base_tok.encode_text(test_sentence).unwrap();
    let trained_tokens = trained_tok.encode_text(test_sentence).unwrap();

    // Base tokenizer uses 1 token per byte
    assert_eq!(base_tokens.len(), test_sentence.len());

    // Trained tokenizer has learned frequent English n-grams; token count must be strictly lower!
    assert!(
        trained_tokens.len() < base_tokens.len(),
        "Trained token count ({}) must be strictly less than base byte count ({})",
        trained_tokens.len(),
        base_tokens.len()
    );

    let _ = std::fs::remove_file(txt_path);
}

#[test]
fn test_4_decode_encode_exact_reproduction_on_multilingual() {
    let (txt_path, jsonl_path) = create_synthetic_corpus("test_4");

    let mut manifest = CorpusManifest::new(42);
    manifest.add_entry(CorpusManifestEntry {
        path: txt_path.clone(),
        weight: 1.0,
        language: None,
        domain: None,
        text_field: None,
    });
    manifest.add_entry(CorpusManifestEntry {
        path: jsonl_path.clone(),
        weight: 1.0,
        language: None,
        domain: None,
        text_field: None,
    });

    let config = TrainingConfig {
        target_vocab_size: 320,
        min_frequency: 2,
        pretokenizer: PretokenizationMode::ScriptAndDigitAware,
        normalization: Normalization::None,
        special_tokens: vec!["<|bos|>".to_string(), "<|eos|>".to_string()],
        seed: 42,
    };

    let (tok, _) = train_from_manifest(&manifest, &config).unwrap();

    let multilingual_tests = vec![
        "English sentence for tokenization.",
        "Türkçe doğal dil işleme ve karakter kümesi: Şemsi Paşa Pasajı.",
        "Русский текст: машинное обучение и нейронные сети.",
        "النص العربي: معالجة اللغة الطبيعية والتعلم الآلي.",
        "中文测试：自回归语言模型分词器。",
        "def hello_world(x: int) -> int:\n\treturn x * 42",
        "\\int_{-\\infty}^\\infty e^{-x^2} dx = \\sqrt{\\pi}",
        "Numbers: 123456789, -0.00145e+10, 0xFF, 2026-11-14",
        "Emojis: 🚀 ✨ 👨‍👩‍👧‍👦 🇹🇷",
    ];

    for sample in multilingual_tests {
        let encoded = tok.encode_text(sample).unwrap();
        let decoded = tok.decode_text(&encoded).unwrap();
        assert_eq!(
            decoded, sample,
            "Trained tokenizer failed exact round-trip reconstruction for: {}",
            sample
        );
    }

    let _ = std::fs::remove_file(txt_path);
    let _ = std::fs::remove_file(jsonl_path);
}

#[test]
fn test_5_deterministic_reproducible_training() {
    let (txt_path, jsonl_path) = create_synthetic_corpus("test_5");

    let txt_clone = txt_path.clone();
    let jsonl_clone = jsonl_path.clone();
    let create_manifest = move || {
        let mut manifest = CorpusManifest::new(999);
        manifest.add_entry(CorpusManifestEntry {
            path: txt_clone.clone(),
            weight: 1.0,
            language: None,
            domain: None,
            text_field: None,
        });
        manifest.add_entry(CorpusManifestEntry {
            path: jsonl_clone.clone(),
            weight: 1.0,
            language: None,
            domain: None,
            text_field: None,
        });
        manifest
    };

    let config = TrainingConfig {
        target_vocab_size: 310,
        min_frequency: 2,
        pretokenizer: PretokenizationMode::ScriptAndDigitAware,
        normalization: Normalization::None,
        special_tokens: vec!["<|bos|>".to_string(), "<|eos|>".to_string()],
        seed: 999,
    };

    // Train run 1
    let (tok1, stats1) = train_from_manifest(&create_manifest(), &config).unwrap();
    // Train run 2 with identical seed & data
    let (tok2, stats2) = train_from_manifest(&create_manifest(), &config).unwrap();

    // 1. Stats must match
    assert_eq!(stats1.merges_learned, stats2.merges_learned);
    assert_eq!(stats1.final_vocab_size, stats2.final_vocab_size);

    // 2. Vocabulary must be identical
    assert_eq!(tok1.vocab_size(), tok2.vocab_size());
    for id in 0..tok1.vocab_size() as u32 {
        assert_eq!(tok1.vocab().get_bytes(id), tok2.vocab().get_bytes(id));
    }

    // 3. Merges must be identical in rank and pairs
    let rules1 = tok1.merges().rules();
    let rules2 = tok2.merges().rules();
    assert_eq!(rules1.len(), rules2.len());
    for i in 0..rules1.len() {
        assert_eq!(rules1[i], rules2[i]);
    }

    let _ = std::fs::remove_file(txt_path);
    let _ = std::fs::remove_file(jsonl_path);
}

#[test]
fn test_6_all_three_pretokenization_strategies_work() {
    let (txt_path, _) = create_synthetic_corpus("test_6");

    let strategies = vec![
        PretokenizationMode::RawByte,
        PretokenizationMode::ConservativeUnicode,
        PretokenizationMode::ScriptAndDigitAware,
    ];

    for strategy in strategies {
        let mut manifest = CorpusManifest::new(42);
        manifest.add_entry(CorpusManifestEntry {
            path: txt_path.clone(),
            weight: 1.0,
            language: None,
            domain: None,
            text_field: None,
        });

        let config = TrainingConfig {
            target_vocab_size: 290,
            min_frequency: 2,
            pretokenizer: strategy,
            normalization: Normalization::None,
            special_tokens: vec!["<|bos|>".to_string(), "<|eos|>".to_string()],
            seed: 42,
        };

        let (tok, stats) = train_from_manifest(&manifest, &config)
            .unwrap_or_else(|e| panic!("Training failed for strategy {:?}: {}", strategy, e));

        assert_eq!(tok.pretokenizer(), strategy);
        assert_eq!(stats.final_vocab_size, 290);

        let test_text = "Machine learning in Türkçe, Русский, and 123 numbers!";
        let encoded = tok.encode_text(test_text).unwrap();
        let decoded = tok.decode_text(&encoded).unwrap();
        assert_eq!(decoded, test_text);
    }

    let _ = std::fs::remove_file(txt_path);
}

#[test]
fn test_7_save_and_load_trained_tokenizer_preserves_behavior() {
    let (txt_path, _) = create_synthetic_corpus("test_7");

    let mut manifest = CorpusManifest::new(42);
    manifest.add_entry(CorpusManifestEntry {
        path: txt_path.clone(),
        weight: 1.0,
        language: None,
        domain: None,
        text_field: None,
    });

    let config = TrainingConfig {
        target_vocab_size: 300,
        min_frequency: 2,
        pretokenizer: PretokenizationMode::ScriptAndDigitAware,
        normalization: Normalization::None,
        special_tokens: vec!["<|bos|>".to_string(), "<|eos|>".to_string()],
        seed: 42,
    };

    let (trained_tok, _) = train_from_manifest(&manifest, &config).unwrap();

    let temp_dir = std::env::temp_dir();
    let model_path = temp_dir.join("saved_trained_model.ntok");

    save_ntok_file(&trained_tok, &model_path).expect("Failed to save trained model");
    let loaded_tok = load_ntok_file(&model_path).expect("Failed to load trained model");

    assert_eq!(trained_tok.vocab_size(), loaded_tok.vocab_size());
    assert_eq!(trained_tok.merges().len(), loaded_tok.merges().len());

    let sample = "<|bos|>Machine learning in Turkish: Yapay zeka!<|eos|>";
    let original_encoded = trained_tok.encode_text(sample).unwrap();
    let loaded_encoded = loaded_tok.encode_text(sample).unwrap();
    assert_eq!(original_encoded, loaded_encoded);

    let decoded = loaded_tok.decode_text(&loaded_encoded).unwrap();
    assert_eq!(decoded, sample);

    let _ = std::fs::remove_file(txt_path);
    let _ = std::fs::remove_file(model_path);
}

#[test]
fn test_8_invalid_corpus_records_reported_correctly() {
    let (_, jsonl_path) = create_synthetic_corpus("test_8");

    let mut manifest = CorpusManifest::new(42);
    manifest.add_entry(CorpusManifestEntry {
        path: jsonl_path.clone(),
        weight: 1.0,
        language: None,
        domain: None,
        text_field: None,
    });

    let config = TrainingConfig {
        target_vocab_size: 280,
        min_frequency: 2,
        pretokenizer: PretokenizationMode::ScriptAndDigitAware,
        normalization: Normalization::None,
        special_tokens: vec!["<|bos|>".to_string()],
        seed: 42,
    };

    let (_, stats) = train_from_manifest(&manifest, &config).unwrap();

    // In create_synthetic_corpus, we explicitly inserted 3 malformed records in the JSONL file:
    // 1. Missing "text" key
    // 2. Invalid non-JSON line
    // 3. "text" value is a number, not a string
    assert_eq!(
        stats.malformed_records_count, 3,
        "Expected exactly 3 malformed records to be detected and reported"
    );

    assert_eq!(stats.malformed_records_sample.len(), 3);
    assert!(
        stats
            .malformed_records_sample
            .iter()
            .any(|s| s.contains("Missing required text field"))
    );
    assert!(
        stats
            .malformed_records_sample
            .iter()
            .any(|s| s.contains("JSON parse error"))
    );
    assert!(
        stats
            .malformed_records_sample
            .iter()
            .any(|s| s.contains("is not a string"))
    );

    let _ = std::fs::remove_file(jsonl_path);
}
