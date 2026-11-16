//! Synthetic-corpus memory profiling harness for the BPE trainer.
//!
//! Generates a synthetic corpus of a requested size, trains a tokenizer, and prints
//! OS-reported resident memory (VmRSS from /proc/self/status) before and after
//! training, alongside unique-word, pair, and occurrence counts from the trainer.
//!
//! Usage:
//!   cargo run --release --example memory_profile -- <records> <target_vocab_size> [--zipf]
//!
//! Default corpus: a Zipf-flavoured template mixture (a small set of common lines plus
//! many rarer indexed lines). `--zipf` uses extra repetition of the common lines.
//!
//! The synthetic corpus deliberately uses index-diversified lines so the unique-word
//! count grows with corpus size; it is NOT a natural-language distribution. Treat
//! results as order-of-magnitude scaling behaviour of this trainer on this machine,
//! not as production sizing guarantees.

use neuratal_tokenizer::{
    CorpusManifest, CorpusManifestEntry, Normalization, PretokenizationMode, TrainingConfig,
    train_from_manifest,
};
use std::io::Write;
use std::time::Instant;

fn current_rss_kb() -> Option<usize> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    for line in status.lines() {
        if let Some(rest) = line.strip_prefix("VmRSS:") {
            return rest.split_whitespace().next()?.parse().ok();
        }
    }
    None
}

/// Generates a unique 10-letter lowercase word for an index (bijective base-26).
fn unique_word(mut n: usize) -> String {
    let mut s = String::with_capacity(10);
    for _ in 0..10 {
        s.push((b'a' + (n % 26) as u8) as char);
        n /= 26;
    }
    s
}

const COMMON_LINES: &[&str] = &[
    "the quick brown fox jumps over the lazy dog",
    "machine learning models process text data efficiently",
    "def compute_gradient(loss, params): return [p.grad for p in params]",
    "numbers: 1 2 3 42 100 999 1000000 3.14159 2.71828 1.41421",
    "multilingual: Bonjour le monde, buongiorno mondo, merhaba dünya",
    "SELECT id, name FROM users WHERE active = 1 ORDER BY name;",
    "mathematical notation: \\int_0^1 x^2 dx = 1/3 and \\sum_{i=1}^n i",
    "common words for this corpus appear very frequently in the data",
];

fn generate_corpus(path: &std::path::Path, records: usize, zipf: bool) {
    let mut f = std::fs::File::create(path).unwrap();
    for i in 0..records {
        if zipf && i % 4 != 0 {
            // 75% of lines drawn from the common set (Zipf-like head)
            let line = COMMON_LINES[i % COMMON_LINES.len()];
            writeln!(f, "common {} {}", i % 97, line).unwrap();
        } else {
            // The remainder: index-diversified (rare) lines carrying a unique long token.
            let unique = unique_word(i);
            writeln!(
                f,
                "document {unique} token-{i:x} payload-{i:o} Δ{i} ∑{i} κ{i} rareword{i}"
            )
            .unwrap();
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let records: usize = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(10_000);
    let target_vocab: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(1_000);
    let zipf = args.iter().any(|a| a == "--zipf");

    let corpus_path = std::env::temp_dir().join(format!(
        "ntok_memory_profile_{}_{}.txt",
        records,
        std::process::id()
    ));
    generate_corpus(&corpus_path, records, zipf);
    let corpus_bytes = std::fs::metadata(&corpus_path).unwrap().len();

    let rss_before = current_rss_kb();

    let mut manifest = CorpusManifest::new(42);
    manifest.add_entry(CorpusManifestEntry {
        path: corpus_path.clone(),
        weight: 1.0,
        language: None,
        domain: None,
        text_field: None,
    });

    let config = TrainingConfig {
        target_vocab_size: target_vocab,
        min_frequency: 2,
        pretokenizer: PretokenizationMode::ConservativeUnicode,
        normalization: Normalization::None,
        special_tokens: vec!["<|bos|>".to_string(), "<|eos|>".to_string()],
        seed: 42,
    };

    let start = Instant::now();
    let (tok, stats) = train_from_manifest(&manifest, &config).expect("training failed");
    let elapsed = start.elapsed();

    let rss_after = current_rss_kb();

    println!("corpus_records={}", records);
    println!("corpus_bytes={}", corpus_bytes);
    println!(
        "corpus_kind={}",
        if zipf {
            "zipf-like"
        } else {
            "uniform-diversified"
        }
    );
    println!("target_vocab_size={}", target_vocab);
    println!("total_records_read={}", stats.total_records_read);
    println!("total_bytes_processed={}", stats.total_bytes_processed);
    println!("unique_words_count={}", stats.unique_words_count);
    println!("initial_pair_count={}", stats.initial_pair_count);
    println!(
        "initial_occurrences_count={}",
        stats.initial_occurrences_count
    );
    println!("merges_learned={}", stats.merges_learned);
    println!(
        "skipped_duplicate_merges={}",
        stats.skipped_duplicate_merges
    );
    println!("final_vocab_size={}", stats.final_vocab_size);
    println!("elapsed_ms={}", elapsed.as_millis());
    println!("rss_before_mb={:?}", rss_before.map(|k| k as f64 / 1024.0));
    println!("rss_after_mb={:?}", rss_after.map(|k| k as f64 / 1024.0));
    match (rss_before, rss_after) {
        (Some(b), Some(a)) if a >= b => {
            println!("rss_growth_mb={:.2}", (a - b) as f64 / 1024.0)
        }
        _ => println!("rss_growth_mb=unknown"),
    }

    let _ = tok.vocab_size();
    let _ = std::fs::remove_file(&corpus_path);
}
