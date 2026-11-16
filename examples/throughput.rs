//! Release-mode throughput measurement for encoding and decoding.
//!
//! Measures encode MB/s, encode tokens/sec, and decode byte reconstruction
//! throughput on a synthetic mixed-language payload built into the binary.
//! Numbers are machine-specific; use for relative comparisons on one machine.
//!
//! Run with:
//!   cargo run --release --example throughput

use neuratal_tokenizer::{
    CorpusManifest, CorpusManifestEntry, Normalization, PretokenizationMode, Tokenizer,
    TrainingConfig, train_from_manifest,
};
use std::time::Instant;

fn sample_payload(repetitions: usize) -> String {
    let mut s = String::with_capacity(repetitions * 220);
    for _ in 0..repetitions {
        s.push_str("The quick brown fox jumps over the lazy dog. ");
        s.push_str("Türkçe: Şemsi Paşa Pasajı'ndan geçtik, 1 2 3 4 5 6 7 8 9 0. ");
        s.push_str("Русский: машинное обучение и нейронные сети в 2026 году. ");
        s.push_str("def solve(x):\n    return x * 42  # programming samples with code\n");
    }
    s
}

fn main() {
    let base = Tokenizer::new_base();
    let payload = sample_payload(2_000); // ~350 KB
    let raw_bytes = payload.as_bytes().to_vec();
    println!("payload: {} bytes", raw_bytes.len());

    // ---- Encode benchmark (base tokenizer, no merges: byte/token 1:1 ceiling) ----
    let iters = 200;
    let start = Instant::now();
    let mut tokens = Vec::new();
    for _ in 0..iters {
        tokens = base.encode_text(&payload).unwrap();
    }
    let dur = start.elapsed().as_secs_f64();
    let bytes = raw_bytes.len() * iters;
    let toks = tokens.len() * iters;
    println!(
        "base encode:      {:.2} MB/s, {:.2}M tokens/s ({} tokens)",
        bytes as f64 / dur / 1e6,
        toks as f64 / dur / 1e6,
        tokens.len()
    );

    // ---- Train a small model to measure trained-model throughput ----
    let corpus_path =
        std::env::temp_dir().join(format!("throughput_corpus_{}.txt", std::process::id()));
    {
        use std::io::Write;
        let mut f = std::fs::File::create(&corpus_path).unwrap();
        for _ in 0..120 {
            writeln!(f, "The quick brown fox jumps over the lazy dog.").unwrap();
            writeln!(
                f,
                "Türkçe: Şemsi Paşa Pasajı'ndan geçtik, 1 2 3 4 5 6 7 8 9 0."
            )
            .unwrap();
            writeln!(
                f,
                "Русский: машинное обучение и нейронные сети в 2026 году."
            )
            .unwrap();
            writeln!(
                f,
                "def solve(x):\n    return x * 42  # programming samples with code"
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
        target_vocab_size: 420,
        min_frequency: 2,
        pretokenizer: PretokenizationMode::ConservativeUnicode,
        normalization: Normalization::None,
        special_tokens: vec!["<|bos|>".to_string(), "<|eos|>".to_string()],
        seed: 42,
    };
    let (trained, _) = train_from_manifest(&manifest, &config).unwrap();
    let _ = std::fs::remove_file(corpus_path);

    let start = Instant::now();
    let mut trained_tokens = Vec::new();
    for _ in 0..iters {
        trained_tokens = trained.encode_text(&payload).unwrap();
    }
    let dur = start.elapsed().as_secs_f64();
    let toks = trained_tokens.len() * iters;
    println!(
        "trained encode:   {:.2} MB/s, {:.2}M tokens/s ({} tokens, compression {:.1} B/token)",
        bytes as f64 / dur / 1e6,
        toks as f64 / dur / 1e6,
        trained_tokens.len(),
        raw_bytes.len() as f64 / trained_tokens.len() as f64
    );

    // ---- Decode benchmark ----
    let start = Instant::now();
    for _ in 0..iters {
        let _ = trained.decode_bytes(&trained_tokens).unwrap();
    }
    let dur = start.elapsed().as_secs_f64();
    println!(
        "trained decode:   {:.2} MB/s (byte reconstruction)",
        bytes as f64 / dur / 1e6
    );

    // ---- Batch encode benchmark (Rayon) ----
    let batch: Vec<&str> = payload.lines().take(64).collect();
    let reps = 30;
    let start = Instant::now();
    let mut total = 0usize;
    for _ in 0..reps {
        for r in trained.encode_batch(&batch) {
            total += r.unwrap().len();
        }
    }
    let dur = start.elapsed().as_secs_f64();
    let batch_bytes: usize = batch.iter().map(|s| s.len()).sum::<usize>() * reps;
    println!(
        "batch encode:     {:.2} MB/s across {} parallel slices ({} tokens in this run)",
        batch_bytes as f64 / dur / 1e6,
        batch.len(),
        total
    );
}
