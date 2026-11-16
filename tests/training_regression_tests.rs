//! Phase 5F/5A training regression tests: large repetitive corpora, multilingual
//! data, rare words, numeric sequences, mixed-script content, sampling
//! determinism, and repeatability.

use neuratal_tokenizer::{
    CorpusManifest, CorpusManifestEntry, Normalization, PretokenizationMode, TrainingConfig,
    load_ntok_file, save_ntok_file, train_from_manifest,
};
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

fn temp_corpus(tag: &str, lines: &[String]) -> PathBuf {
    let id = COUNTER.fetch_add(1, Ordering::SeqCst);
    let path = std::env::temp_dir().join(format!(
        "ntok_training_{}_{}_{}.txt",
        tag,
        std::process::id(),
        id
    ));
    let mut f = std::fs::File::create(&path).unwrap();
    for line in lines {
        writeln!(f, "{}", line).unwrap();
    }
    path
}

fn manifest_for(path: PathBuf, seed: u64) -> CorpusManifest {
    let mut manifest = CorpusManifest::new(seed);
    manifest.add_entry(CorpusManifestEntry {
        path,
        weight: 1.0,
        language: None,
        domain: None,
        text_field: None,
    });
    manifest
}

fn base_config(target_vocab_size: usize, seed: u64) -> TrainingConfig {
    TrainingConfig {
        target_vocab_size,
        min_frequency: 2,
        pretokenizer: PretokenizationMode::ConservativeUnicode,
        normalization: Normalization::None,
        special_tokens: vec!["<|bos|>".to_string(), "<|eos|>".to_string()],
        seed,
    }
}

#[test]
fn training_on_large_repetitive_corpus() {
    let lines: Vec<String> = (0..600)
        .map(|i| {
            format!(
                "record {} the quick brown fox jumps over the lazy dog",
                i % 37
            )
        })
        .collect();
    let corpus = temp_corpus("large_repetitive", &lines);

    let config = base_config(340, 42);
    let (tok, stats) = train_from_manifest(&manifest_for(corpus.clone(), 42), &config).unwrap();

    assert_eq!(stats.total_records_read, 600);
    assert!(stats.total_bytes_processed > 0);
    // The tiny 37-line cycle saturates: training stops when no pair reaches
    // min_frequency, which is expected behavior for small corpora.
    assert!(stats.final_vocab_size >= 258);
    assert!(stats.merges_learned > 0);
    assert_eq!(stats.final_vocab_size, tok.vocab_size());

    let sample = "the quick brown fox";
    let encoded = tok.encode_text(sample).unwrap();
    let decoded = tok.decode_text(&encoded).unwrap();
    assert_eq!(decoded, sample);

    let _ = std::fs::remove_file(corpus);
}

#[test]
fn training_with_multilingual_corpus() {
    let mut lines = Vec::new();
    for _ in 0..8 {
        lines.push(
            "English: machine learning transforms natural language processing pipelines today"
                .to_string(),
        );
        lines.push(
            "Türkçe: derin öğrenme modelleri çok dilli metinleri işler ve sonuçlar üretir"
                .to_string(),
        );
        lines.push(
            "Русский: модели машинного обучения обрабатывают многоязычные текстовые данные"
                .to_string(),
        );
        lines.push(
            "العربية: نماذج التعلم العميق تعالج النصوص متعددة اللغات بكفاءة عالية".to_string(),
        );
        lines.push("हिन्दी: मशीन लर्निंग मॉडल बहुभाषी पाठ डेटा को कुशलतापूर्वक संसाधित करते हैं".to_string());
        lines.push("中文：深度学习模型高效地处理多语言文本数据和各种语料库".to_string());
        lines.push("日本語：深層学習モデルは多言語テキストデータを効率的に処理します".to_string());
        lines
            .push("한국어: 딥러닝 모델은 다국어 텍스트 데이터를 효율적으로 처리합니다".to_string());
    }
    let corpus = temp_corpus("multilingual", &lines);

    let config = base_config(400, 42);
    let (tok, stats) = train_from_manifest(&manifest_for(corpus.clone(), 42), &config).unwrap();

    assert_eq!(stats.final_vocab_size, 400);

    for sample in [
        "Türkçe: derin öğrenme modelleri",
        "Русский: модели машинного обучения",
        "العربية: نماذج التعلم العميق",
        "中文：深度学习模型",
        "日本語：深層学習モデル",
    ] {
        let encoded = tok.encode_text(sample).unwrap();
        let decoded = tok.decode_text(&encoded).unwrap();
        assert_eq!(decoded, sample);
    }

    let _ = std::fs::remove_file(corpus);
}

#[test]
fn training_with_rare_words_respects_min_frequency() {
    // The rare word appears only once; its pairs can never reach min_frequency = 2,
    // while common pairs are abundant. Training must succeed and stay deterministic.
    let mut lines: Vec<String> = Vec::new();
    for _ in 0..12 {
        lines.push("common common common common common words words words".to_string());
    }
    lines.push("supercalifragilisticexpialidocious".to_string()); // singleton rare word
    let corpus = temp_corpus("rare_words", &lines);

    let config = base_config(280, 42);
    let (tok1, stats1) = train_from_manifest(&manifest_for(corpus.clone(), 42), &config).unwrap();
    let (tok2, stats2) = train_from_manifest(&manifest_for(corpus.clone(), 42), &config).unwrap();

    assert_eq!(tok1.merges().rules(), tok2.merges().rules());
    assert_eq!(stats1.merges_learned, stats2.merges_learned);

    // Round trip on the rare word must still work via byte fallback.
    let sample = "supercalifragilisticexpialidocious";
    let encoded = tok1.encode_text(sample).unwrap();
    let decoded = tok1.decode_text(&encoded).unwrap();
    assert_eq!(decoded, sample);

    let _ = std::fs::remove_file(corpus);
}

#[test]
fn training_with_min_frequency_above_all_pairs_learns_nothing() {
    let lines = vec!["tiny corpus with few occurrences".to_string()];
    let corpus = temp_corpus("high_min_freq", &lines);

    let config = TrainingConfig {
        target_vocab_size: 300,
        min_frequency: 100, // unreachable for this corpus
        pretokenizer: PretokenizationMode::ConservativeUnicode,
        normalization: Normalization::None,
        special_tokens: vec!["<|bos|>".to_string(), "<|eos|>".to_string()],
        seed: 42,
    };
    let (tok, stats) = train_from_manifest(&manifest_for(corpus.clone(), 42), &config).unwrap();

    assert_eq!(stats.merges_learned, 0);
    // 256 base + 0 merges + 2 specials
    assert_eq!(tok.vocab_size(), 258);

    let _ = std::fs::remove_file(corpus);
}

#[test]
fn training_with_numeric_sequences() {
    let mut lines = Vec::new();
    for i in 0..40 {
        lines.push(format!(
            "value = {i}.{i:03}e+{i}; hex = 0x{i:X}{i:X}; bin = 0b{i:b}{i:b}"
        ));
        lines.push(format!(
            "timestamp 2026-{:02}-{:02}T{:02}:59:59.999Z",
            i % 28 + 1,
            i % 28 + 1,
            i % 24
        ));
    }
    let corpus = temp_corpus("numeric", &lines);

    let config = base_config(360, 42);
    let (tok, _) = train_from_manifest(&manifest_for(corpus.clone(), 42), &config).unwrap();

    let sample = "2026-11-14T23:59:59.999Z 0xDEADBEEF 6.02214076e+23";
    let encoded = tok.encode_text(sample).unwrap();
    let decoded = tok.decode_text(&encoded).unwrap();
    assert_eq!(decoded, sample);

    let _ = std::fs::remove_file(corpus);
}

#[test]
fn training_with_mixed_script_and_code() {
    let mut lines = Vec::new();
    for _ in 0..10 {
        lines.push(
            "def tokenize(text: str) -> list[int]:\n    return [ord(c) for c in text]".to_string(),
        );
        lines.push("Türkçe yorum: bu fonksiyon rust ile yazılmıştır ve hızlıdır".to_string());
        lines.push("```python\nprint('merhaba dünya 🌍')\n```".to_string());
        lines
            .push("SELECT id, COUNT(*) FROM logs WHERE ts > '2026-01-01' GROUP BY id;".to_string());
        lines.push("수식: ∫₀¹ x² dx = 1/3, ∑ⁿᵢ₌₁ i = n(n+1)/2".to_string());
    }
    let corpus = temp_corpus("mixed_script_code", &lines);

    let config = base_config(380, 42);
    let (tok, _) = train_from_manifest(&manifest_for(corpus.clone(), 42), &config).unwrap();

    let sample = "def f():\n    return 'merhaba dünya 🌍'";
    let encoded = tok.encode_text(sample).unwrap();
    let decoded = tok.decode_text(&encoded).unwrap();
    assert_eq!(decoded, sample);

    let _ = std::fs::remove_file(corpus);
}

#[test]
fn weighted_sampling_is_deterministic_across_runs() {
    // Entry with weight < 1.0 uses deterministic per-record sampling.
    let lines: Vec<String> = (0..80)
        .map(|i| format!("line {i} with some reasonably repetitive sample content for sampling"))
        .collect();
    let corpus = temp_corpus("weighted_sampling", &lines);

    let mut manifest = CorpusManifest::new(1234);
    manifest.add_entry(CorpusManifestEntry {
        path: corpus.clone(),
        weight: 0.5, // subsample deterministically
        language: None,
        domain: None,
        text_field: None,
    });

    let config = base_config(280, 1234);
    let (tok1, stats1) = train_from_manifest(&manifest, &config).unwrap();
    let (tok2, stats2) = train_from_manifest(&manifest, &config).unwrap();

    assert_eq!(stats1.total_records_read, stats2.total_records_read);
    assert_eq!(stats1.total_bytes_processed, stats2.total_bytes_processed);
    assert_eq!(stats1.unique_words_count, stats2.unique_words_count);
    assert_eq!(tok1.merges().rules(), tok2.merges().rules());

    let _ = std::fs::remove_file(corpus);
}

#[test]
fn repeated_serialization_is_byte_deterministic() {
    let lines: Vec<String> = (0..30)
        .map(|i| {
            format!(
                "determinism check line number {i} with repeated content like apples apples apples"
            )
        })
        .collect();
    let corpus = temp_corpus("serialization_determinism", &lines);

    let config = base_config(300, 42);
    let (tok, _) = train_from_manifest(&manifest_for(corpus.clone(), 42), &config).unwrap();

    let id = COUNTER.fetch_add(1, Ordering::SeqCst);
    let p1 = std::env::temp_dir().join(format!("ntok_det_a_{}_{}.ntok", std::process::id(), id));
    let p2 = std::env::temp_dir().join(format!("ntok_det_b_{}_{}.ntok", std::process::id(), id));

    save_ntok_file(&tok, &p1).unwrap();
    save_ntok_file(&tok, &p2).unwrap();

    let bytes1 = std::fs::read(&p1).unwrap();
    let bytes2 = std::fs::read(&p2).unwrap();
    assert_eq!(
        bytes1, bytes2,
        "repeated serialization of the same model must be byte-identical"
    );

    let loaded = load_ntok_file(&p1).unwrap();
    assert_eq!(loaded.merges().rules(), tok.merges().rules());

    let _ = std::fs::remove_file(p1);
    let _ = std::fs::remove_file(p2);
    let _ = std::fs::remove_file(corpus);
}
