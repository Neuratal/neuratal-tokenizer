//! Native held-out benchmark harness for NTokenizer `.ntok` models.
//!
//! Measures byte-weighted and equal-category bytes/token, token usage,
//! encode/decode throughput, per-file latency, and peak RSS. Corpus files are
//! preloaded so filesystem I/O is excluded from the measured encode window.
//!
//! Example:
//!   RAYON_NUM_THREADS=1 cargo run --release --example benchmark_ntok -- \
//!     --model output/v0.3/ntokenizer-0.3-256k.ntok \
//!     --manifest data/v0.3/manifest/manifest_benchmark.json \
//!     --repeats 5 --warmup 1 \
//!     --out reports/v0.3/benchmark_ntok__single.json

use rayon::prelude::*;
use serde::Deserialize;
use serde_json::json;
use std::collections::{BTreeMap, HashSet};
use std::path::PathBuf;
use std::time::Instant;

use neuratal_tokenizer::{TokenId, load_ntok_file, sha256_hex};

#[derive(Debug, Deserialize)]
struct BenchManifest {
    entries: Vec<BenchEntry>,
}

#[derive(Debug, Deserialize)]
struct BenchEntry {
    path: PathBuf,
    #[serde(default)]
    bucket: Option<String>,
    #[serde(default)]
    domain: Option<String>,
}

#[derive(Debug, Clone, Default)]
struct BucketStats {
    files: u64,
    bytes: u64,
    tokens: u64,
    words: u64,
    decode_ok: u64,
    decode_fail: u64,
}

#[derive(Debug, Clone)]
struct UsedBits {
    words: Vec<u64>,
}

impl UsedBits {
    fn new(vocab: usize) -> Self {
        Self {
            words: vec![0u64; vocab.div_ceil(64)],
        }
    }

    #[inline]
    fn set(&mut self, id: TokenId) {
        let id = id as usize;
        if id / 64 < self.words.len() {
            self.words[id / 64] |= 1u64 << (id % 64);
        }
    }

    fn merge(&mut self, other: &Self) {
        for (a, b) in self.words.iter_mut().zip(other.words.iter()) {
            *a |= *b;
        }
    }

    fn count(&self) -> u64 {
        self.words.iter().map(|w| w.count_ones() as u64).sum()
    }
}

#[derive(Debug, Clone)]
struct LocalStats {
    buckets: BTreeMap<String, BucketStats>,
    used: UsedBits,
    durations: Vec<f64>,
    per_file: Vec<serde_json::Value>,
}

impl LocalStats {
    fn new(vocab: usize) -> Self {
        Self {
            buckets: BTreeMap::new(),
            used: UsedBits::new(vocab),
            durations: Vec::new(),
            per_file: Vec::new(),
        }
    }

    fn merge(&mut self, other: LocalStats) {
        for (bucket, stats) in other.buckets {
            let agg = self.buckets.entry(bucket).or_default();
            agg.files += stats.files;
            agg.bytes += stats.bytes;
            agg.tokens += stats.tokens;
            agg.words += stats.words;
            agg.decode_ok += stats.decode_ok;
            agg.decode_fail += stats.decode_fail;
        }
        self.used.merge(&other.used);
        self.durations.extend(other.durations);
        self.per_file.extend(other.per_file);
    }

    fn totals(&self) -> (u64, u64, u64, u64, u64) {
        let mut bytes = 0;
        let mut tokens = 0;
        let mut words = 0;
        let mut ok = 0;
        let mut fail = 0;
        for stats in self.buckets.values() {
            bytes += stats.bytes;
            tokens += stats.tokens;
            words += stats.words;
            ok += stats.decode_ok;
            fail += stats.decode_fail;
        }
        (bytes, tokens, words, ok, fail)
    }
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let rank = ((sorted.len() - 1) as f64 * p).round() as usize;
    sorted[rank]
}

fn peak_rss_kb() -> u64 {
    let Ok(status) = std::fs::read_to_string("/proc/self/status") else {
        return 0;
    };
    for line in status.lines() {
        if let Some(rest) = line.strip_prefix("VmHWM:") {
            if let Some(value) = rest.split_whitespace().next() {
                return value.parse().unwrap_or(0);
            }
        }
    }
    0
}

fn main() {
    let mut args = std::env::args().skip(1);
    let mut model_path: Option<PathBuf> = None;
    let mut manifest_path: Option<PathBuf> = None;
    let mut out_path: Option<PathBuf> = None;
    let mut label = String::new();
    let mut repeats = 5usize;
    let mut warmup = 1usize;
    let mut decode_check = true;
    let mut per_file_out: Option<PathBuf> = None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--model" => model_path = args.next().map(PathBuf::from),
            "--manifest" => manifest_path = args.next().map(PathBuf::from),
            "--out" => out_path = args.next().map(PathBuf::from),
            "--label" => label = args.next().unwrap_or_default(),
            "--repeats" => repeats = args.next().and_then(|v| v.parse().ok()).unwrap_or(5),
            "--warmup" => warmup = args.next().and_then(|v| v.parse().ok()).unwrap_or(1),
            "--no-decode" => decode_check = false,
            "--per-file-json" => per_file_out = args.next().map(PathBuf::from),
            other => {
                eprintln!("unknown argument: {other}");
                std::process::exit(2);
            }
        }
    }

    let model_path = model_path.expect("--model is required");
    let manifest_path = manifest_path.expect("--manifest is required");
    if label.is_empty() {
        label = model_path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "model".to_string());
    }

    let load_start = Instant::now();
    let tok = load_ntok_file(&model_path).expect("failed to load model");
    let load_seconds = load_start.elapsed().as_secs_f64();
    let model_bytes = std::fs::read(&model_path).expect("failed to read model bytes");
    let model_sha256 = sha256_hex(&model_bytes);
    let vocab_size = tok.vocab_size();
    let pretokenizer = tok.pretokenizer();

    let manifest_text = std::fs::read_to_string(&manifest_path).expect("failed to read manifest");
    let manifest: BenchManifest = serde_json::from_str(&manifest_text)
        .unwrap_or_else(|e| panic!("failed to parse manifest {}: {e}", manifest_path.display()));

    let mut seen = HashSet::new();
    let mut files: Vec<(String, PathBuf, Vec<u8>, u64)> = Vec::with_capacity(manifest.entries.len());
    for entry in &manifest.entries {
        if !seen.insert(entry.path.clone()) {
            panic!("duplicate manifest entry: {}", entry.path.display());
        }
        let bucket = entry
            .bucket
            .clone()
            .or_else(|| entry.domain.clone())
            .unwrap_or_else(|| "other".to_string());
        let bytes = std::fs::read(&entry.path)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", entry.path.display()));
        let words = std::str::from_utf8(&bytes)
            .map(|s| s.split_whitespace().count() as u64)
            .unwrap_or(0);
        files.push((bucket, entry.path.clone(), bytes, words));
    }
    if files.is_empty() {
        panic!("manifest has no entries");
    }
    let total_bytes: u64 = files.iter().map(|f| f.2.len() as u64).sum();
    let total_words: u64 = files.iter().map(|f| f.3).sum();
    eprintln!(
        "loaded {} files, {:.1} MB, {} vocab, {:?} from {}",
        files.len(),
        total_bytes as f64 / 1e6,
        vocab_size,
        pretokenizer,
        model_path.display()
    );

    let thread_count = rayon::current_num_threads();
    let mut run_summaries: Vec<serde_json::Value> = Vec::new();
    let mut first_measured: Option<LocalStats> = None;
    let mut token_signature: Option<u64> = None;
    let mut warmup_seconds = 0.0f64;

    for run in 0..(warmup + repeats) {
        let measured = run >= warmup;
        let capture = run == warmup;
        let start = Instant::now();
        let partial = files
            .par_iter()
            .fold(
                || LocalStats::new(vocab_size),
                |mut acc, (bucket, path, bytes, words)| {
                    let t0 = Instant::now();
                    let ids = tok.encode_bytes(bytes);
                    let elapsed = t0.elapsed().as_secs_f64();
                    if capture {
                        for &id in &ids {
                            acc.used.set(id);
                        }
                    }
                    acc.durations.push(elapsed);
                    let stats = acc.buckets.entry(bucket.clone()).or_default();
                    stats.files += 1;
                    stats.bytes += bytes.len() as u64;
                    stats.tokens += ids.len() as u64;
                    stats.words += *words;
                    acc.per_file.push(serde_json::json!({
                        "path": path.to_string_lossy(),
                        "bucket": bucket,
                        "bytes": bytes.len(),
                        "tokens": ids.len(),
                        "words": words,
                    }));
                    acc
                },
            )
            .reduce(|| LocalStats::new(vocab_size), |mut a, b| {
                a.merge(b);
                a
            });
        let elapsed = start.elapsed().as_secs_f64();
        let (bytes, tokens, words, _ok, _fail) = partial.totals();
        let signature = bytes ^ tokens.rotate_left(17) ^ words.rotate_left(33);
        match token_signature {
            None => token_signature = Some(signature),
            Some(expected) if expected != signature => {
                panic!("non-deterministic token counts across repeats")
            }
            _ => {}
        }
        if measured {
            if first_measured.is_none() {
                first_measured = Some(partial.clone());
            }
            run_summaries.push(json!({
                "repeat": run + 1 - warmup,
                "seconds": elapsed,
                "bytes": bytes,
                "tokens": tokens,
                "mb_per_s": bytes as f64 / elapsed.max(1e-12) / 1e6,
                "tokens_per_s": tokens as f64 / elapsed.max(1e-12),
                "peak_rss_kb": peak_rss_kb(),
            }));
            eprintln!(
                "repeat {}/{}: {:.3} s, {:.2} MB/s, {:.0} tok/s",
                run + 1 - warmup,
                repeats,
                elapsed,
                bytes as f64 / elapsed.max(1e-12) / 1e6,
                tokens as f64 / elapsed.max(1e-12)
            );
        } else {
            warmup_seconds = elapsed;
        }
    }

    let measured = first_measured.expect("no measured repeat");

    if let Some(out_path) = per_file_out {
        let body = serde_json::to_vec(&measured.per_file).expect("serialize per-file json");
        std::fs::write(&out_path, &body).expect("failed to write per-file json");
        eprintln!("wrote per-file json {}", out_path.display());
    }
    let (bytes, tokens, words, _, _) = measured.totals();
    let mut bucket_json = serde_json::Map::new();
    let mut equal_category_bpt = 0.0f64;
    let mut category_count = 0usize;
    for (bucket, stats) in &measured.buckets {
        let bpt = stats.bytes as f64 / stats.tokens.max(1) as f64;
        equal_category_bpt += bpt;
        category_count += 1;
        bucket_json.insert(
            bucket.clone(),
            json!({
                "files": stats.files,
                "bytes": stats.bytes,
                "tokens": stats.tokens,
                "words": stats.words,
                "bytes_per_token": bpt,
                "tokens_per_byte": stats.tokens as f64 / stats.bytes.max(1) as f64,
                "tokens_per_word": stats.tokens as f64 / stats.words.max(1) as f64,
            }),
        );
    }
    equal_category_bpt /= category_count.max(1) as f64;

    // Decode validity and throughput measured separately so encode numbers are
    // not contaminated by reconstructing bytes.
    let mut decode_seconds = 0.0f64;
    let mut decode_ok = 0u64;
    let mut decode_fail = 0u64;
    if decode_check {
        // Encode outside the timed window so `decode.mb_per_s` measures decoding
        // only. Holding the benchmark's token IDs in memory is acceptable
        // (544 files, ~170 MB for a 42M-token model).
        let all_ids: Vec<Vec<neuratal_tokenizer::TokenId>> = files
            .par_iter()
            .map(|(_bucket, _path, bytes, _words)| tok.encode_bytes(bytes))
            .collect();
        let start = Instant::now();
        let (ok, fail) = all_ids
            .par_iter()
            .zip(files.par_iter())
            .map(|(ids, (_bucket, _path, bytes, _words))| match tok.decode_bytes(ids) {
                Ok(decoded) => {
                    if decoded == *bytes {
                        (1u64, 0u64)
                    } else {
                        (0u64, 1u64)
                    }
                }
                Err(_) => (0u64, 1u64),
            })
            .reduce(|| (0u64, 0u64), |a, b| (a.0 + b.0, a.1 + b.1));
        decode_seconds = start.elapsed().as_secs_f64();
        decode_ok = ok;
        decode_fail = fail;
    }

    let mut durations = measured.durations.clone();
    durations.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let median_ms = percentile(&durations, 0.50) * 1e3;
    let p95_ms = percentile(&durations, 0.95) * 1e3;
    let max_ms = durations.last().copied().unwrap_or(0.0) * 1e3;

    let seconds: Vec<f64> = run_summaries
        .iter()
        .filter_map(|v| v.get("seconds").and_then(|x| x.as_f64()))
        .collect();
    let mut sorted_seconds = seconds.clone();
    sorted_seconds.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let median_seconds = percentile(&sorted_seconds, 0.50);
    let median_mb_s = bytes as f64 / median_seconds.max(1e-12) / 1e6;
    let median_tokens_s = tokens as f64 / median_seconds.max(1e-12);
    let used_vocab = measured.used.count();

    let metadata = tok.metadata().map(|m| {
        json!({
            "model_name": m.model_name,
            "model_version": m.model_version,
            "format_version": m.format_version,
            "pretokenizer_mode": m.pretokenizer_mode,
            "min_frequency": m.min_frequency,
            "seed": m.seed,
            "corpus_bytes": m.corpus_bytes,
            "corpus_records": m.corpus_records,
            "training_started_at": m.training_started_at,
            "training_finished_at": m.training_finished_at,
            "training_cli": m.training_cli,
            "corpus_manifest_sha256": m.corpus_manifest_sha256,
        })
    });

    let result = json!({
        "label": label,
        "model_path": model_path,
        "model_sha256": model_sha256,
        "model_bytes": model_bytes.len(),
        "vocab_size": vocab_size,
        "merges": tok.merges().len(),
        "pretokenizer": format!("{pretokenizer:?}"),
        "metadata": metadata,
        "manifest": manifest_path,
        "threads": thread_count,
        "warmup_seconds": warmup_seconds,
        "repeats": run_summaries,
        "median_seconds": median_seconds,
        "median_mb_per_s": median_mb_s,
        "median_tokens_per_s": median_tokens_s,
        "decode": {
            "checked": decode_check,
            "ok_files": decode_ok,
            "failed_files": decode_fail,
            "seconds": decode_seconds,
            "mb_per_s": if decode_check { total_bytes as f64 / decode_seconds.max(1e-12) / 1e6 } else { 0.0 },
        },
        "latency": {
            "scope": "per-file wall time within the measured pass (parallel contention applies when threads > 1)",
            "median_ms": median_ms,
            "p95_ms": p95_ms,
            "max_ms": max_ms,
        },
        "peak_rss_kb": peak_rss_kb(),
        "load_seconds": load_seconds,
        "totals": {
            "files": files.len(),
            "bytes": bytes,
            "tokens": tokens,
            "words": words,
            "bytes_per_token": bytes as f64 / tokens.max(1) as f64,
            "tokens_per_byte": tokens as f64 / bytes.max(1) as f64,
            "tokens_per_word": tokens as f64 / words.max(1) as f64,
            "equal_category_bytes_per_token": equal_category_bpt,
            "categories": category_count,
            "used_vocab": used_vocab,
            "unused_vocab": vocab_size as u64 - used_vocab,
            "vocab_usage_pct": used_vocab as f64 / vocab_size.max(1) as f64 * 100.0,
            "manifest_total_words": total_words,
        },
        "buckets": bucket_json,
    });

    if let Some(out) = out_path {
        if let Some(parent) = out.parent() {
            std::fs::create_dir_all(parent).expect("failed to create output directory");
        }
        std::fs::write(&out, serde_json::to_string_pretty(&result).unwrap())
            .expect("failed to write output");
        eprintln!("wrote {}", out.display());
    }

    println!("model: {label}");
    println!(
        "bytes/token: {:.4}  equal-category: {:.4}  tokens: {}  used vocab: {}/{} ({:.2}%)",
        bytes as f64 / tokens.max(1) as f64,
        equal_category_bpt,
        tokens,
        used_vocab,
        vocab_size,
        used_vocab as f64 / vocab_size.max(1) as f64 * 100.0
    );
    println!(
        "encode: {:.2} MB/s  {:.0} tok/s  median {:.3} s  peak RSS {:.1} MiB",
        median_mb_s,
        median_tokens_s,
        median_seconds,
        peak_rss_kb() as f64 / 1024.0
    );
    if decode_check {
        println!(
            "decode: {:.2} MB/s  exact {}/{} files",
            total_bytes as f64 / decode_seconds.max(1e-12) / 1e6,
            decode_ok,
            decode_ok + decode_fail
        );
    }
    println!("latency: median {:.3} ms  p95 {:.3} ms  max {:.3} ms", median_ms, p95_ms, max_ms);
}
