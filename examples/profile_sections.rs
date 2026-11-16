//! Section-level timing of the encode path: pretokenizer vs total vs BPE model.

use neuratal_tokenizer::{PretokenizationMode, load_ntok_file};
use std::time::Instant;

fn pick_file(manifest_path: &str) -> Vec<u8> {
    let text = std::fs::read_to_string(manifest_path).unwrap();
    let manifest: serde_json::Value = serde_json::from_str(&text).unwrap();
    let mut best = (0usize, String::new());
    for e in manifest["entries"].as_array().unwrap() {
        let b = e["bytes"].as_u64().unwrap_or(0) as usize;
        if b > best.0 {
            best = (b, e["path"].as_str().unwrap().to_string());
        }
    }
    eprintln!("largest file: {} ({} bytes)", best.1, best.0);
    std::fs::read(&best.1).unwrap()
}

fn main() {
    let model_path = std::env::args().nth(1).expect("model");
    let manifest = std::env::args().nth(2).expect("manifest");
    let tok = load_ntok_file(&model_path).unwrap();
    let bytes = pick_file(&manifest);
    let text = String::from_utf8_lossy(&bytes).into_owned();

    let iters = 10;
    for (label, mode) in [
        ("MathSymbolAware (v0.4)", PretokenizationMode::MathSymbolAware),
        ("LeadingWhitespaceUnicode (v0.2)", PretokenizationMode::LeadingWhitespaceUnicode),
    ] {
        // Pretokenizer-only timing
        let start = Instant::now();
        let mut total_pieces = 0usize;
        for _ in 0..iters {
            total_pieces += mode.split_text(&text).len();
        }
        let pre_ms = start.elapsed().as_secs_f64() * 1000.0 / iters as f64;

        // Total encode timing (same machine, same file)
        let start = Instant::now();
        let mut tokens = 0usize;
        for _ in 0..iters {
            tokens = tok.encode_bytes(&bytes).len();
        }
        let total_ms = start.elapsed().as_secs_f64() * 1000.0 / iters as f64;

        println!("{label}");
        println!("  bytes {:.1} MB", bytes.len() as f64 / 1e6);
        println!("  pretokenize {:.1} ms  ({:.2} MB/s)", pre_ms, bytes.len() as f64 / pre_ms / 1000.0);
        println!("  encode       {:.1} ms  ({:.2} MB/s)", total_ms, bytes.len() as f64 / total_ms / 1000.0);
        println!("  model/BPE share: {:.1}%", (total_ms - pre_ms) / total_ms * 100.0);
        println!("  pre-token pieces: {}", total_pieces / iters);
        println!("  tokens: {}", tokens);
    }
}
