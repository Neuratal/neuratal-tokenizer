//! Phase 5 Memory Profiling & Scaling Tests
//!
//! Measures actual OS-reported resident set size (VmRSS), word counts,
//! pair frequencies, occurrences, and training throughput on progressively
//! larger synthetic corpora.

use neuratal_tokenizer::{
    CorpusManifest, CorpusManifestEntry, Normalization, PretokenizationMode, TrainingConfig,
    train_from_manifest,
};
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

/// Reads current resident set size (VmRSS) in kilobytes from /proc/self/status on Linux.
fn get_current_rss_kb() -> usize {
    if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
        for line in status.lines() {
            if line.starts_with("VmRSS:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2
                    && let Ok(kb) = parts[1].parse::<usize>()
                {
                    return kb;
                }
            }
        }
    }
    0
}

fn generate_scalable_corpus(path: &PathBuf, num_records: usize) {
    let mut f = File::create(path).unwrap();
    let samples = [
        "Autoregressive language models predict token probability distributions.",
        "Yapay zeka ve derin öğrenme modelleri doğal dil işleme görevlerini yerine getirir.",
        "Алгоритмы машинного обучения оптимизируют параметры нейронных сетей.",
        "تعتمد معالجة اللغة الطبيعية على خوارزميات التجزئة الفعالة للبيانات.",
        "深度学习自然语言处理模型利用多语言分词技术实现高效的序列建模。",
        "def compute_gradients(loss, params):\n    return [p.grad for p in params]",
        "\\int_{-\\infty}^\\infty e^{-x^2} dx = \\sqrt{\\pi}, \\quad \\sum_{i=1}^n i = \\frac{n(n+1)}{2}",
        "Numbers: 123456789, -3.14159, 1.25e+10, 0xABCDEF, 2026-11-14",
        "Emojis: 👨‍👩‍👧‍👦 🚀 🇹🇷 1️⃣ 👍🏽 ❤️ ✨",
    ];

    for i in 0..num_records {
        let sample = samples[i % samples.len()];
        writeln!(f, "Record {} | {}", i, sample).unwrap();
    }
}

#[test]
fn test_memory_profiling_progressively_larger_corpora() {
    let temp_dir = std::env::temp_dir();
    let scales = [500, 2_500, 10_000];

    println!(
        "\n========================================================================================="
    );
    println!(
        "NEURATAL TOKENIZER MEMORY PROFILING & SCALING MEASUREMENTS (/proc/self/status VmRSS)"
    );
    println!(
        "========================================================================================="
    );
    println!(
        "{:<10} | {:<12} | {:<12} | {:<14} | {:<14} | {:<12} | {:<10}",
        "Records",
        "Corpus Bytes",
        "Unique Words",
        "Active Pairs",
        "Occurrences",
        "Peak RSS (MB)",
        "Time (ms)"
    );
    println!(
        "{:-<10}-+-{:-<12}-+-{:-<12}-+-{:-<14}-+-{:-<14}-+-{:-<12}-+-{:-<10}",
        "", "", "", "", "", "", ""
    );

    for &num_records in &scales {
        let corpus_path = temp_dir.join(format!(
            "scale_corpus_{}_{}.txt",
            num_records,
            std::process::id()
        ));
        generate_scalable_corpus(&corpus_path, num_records);

        let mut manifest = CorpusManifest::new(42);
        manifest.add_entry(CorpusManifestEntry {
            path: corpus_path.clone(),
            weight: 1.0,
            language: None,
            domain: None,
            text_field: None,
        });

        let config = TrainingConfig {
            target_vocab_size: 400,
            min_frequency: 2,
            pretokenizer: PretokenizationMode::ConservativeUnicode,
            normalization: Normalization::None,
            special_tokens: vec!["<|bos|>".to_string(), "<|eos|>".to_string()],
            seed: 42,
        };

        let _rss_before = get_current_rss_kb();
        let (tok, stats) = train_from_manifest(&manifest, &config).unwrap();
        let rss_after = get_current_rss_kb();

        assert_eq!(stats.final_vocab_size, 400);
        assert_eq!(tok.vocab_size(), 400);

        let rss_mb = rss_after as f64 / 1024.0;

        println!(
            "{:<10} | {:<12} | {:<12} | {:<14} | {:<14} | {:<12.2} | {:<10}",
            stats.total_records_read,
            stats.total_bytes_processed,
            stats.unique_words_count,
            stats.initial_pair_count,
            stats.initial_occurrences_count,
            rss_mb,
            stats.elapsed_millis,
        );

        let _ = std::fs::remove_file(corpus_path);
    }
}
