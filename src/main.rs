//! Neuratal Tokenizer CLI application.

use clap::{Parser, Subcommand};
use neuratal_tokenizer::{
    CorpusManifest, CorpusManifestEntry, ModelMetadata, Normalization, PretokenizationMode,
    Tokenizer, TrainingConfig, load_ntok_file, save_ntok_file, sha256_hex,
    train_from_manifest_with_progress_and_script_aware_min_word_freq, validate_tokenizer,
};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// UTC timestamp as `YYYY-MM-DDTHH:MM:SSZ` (dependency-free civil-from-days conversion).
fn utc_now_iso8601() -> String {
    fn civil_from_days(z: i64) -> (i64, u64, u64) {
        let z = z + 719468;
        let era = if z >= 0 { z } else { z - 146096 } / 146097;
        let doe = (z - era * 146097) as u64; // [0, 146096]
        let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365; // [0, 399]
        let y = yoe as i64 + era * 400;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
        let mp = (5 * doy + 2) / 153; // [0, 11]
        let d = doy - (153 * mp + 2) / 5 + 1; // [1, 31]
        let m = if mp < 10 { mp + 3 } else { mp - 9 }; // [1, 12]
        (if m <= 2 { y + 1 } else { y }, m, d)
    }
    let now = std::env::var("SOURCE_DATE_EPOCH")
        .ok()
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or_else(|| {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64
        });
    let days = now.div_euclid(86_400);
    let secs = now.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year,
        month,
        day,
        secs / 3600,
        (secs % 3600) / 60,
        secs % 60
    )
}

/// Best-effort `git rev-parse HEAD`; empty string when unavailable (e.g. unborn branch).
fn git_commit_hash() -> String {
    std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}

/// SHA-256 of a file's bytes, empty string on read failure.
fn file_sha256_hex(path: &std::path::Path) -> String {
    std::fs::read(path).map(|b| sha256_hex(&b)).unwrap_or_default()
}

#[derive(Parser)]
#[command(name = "ntokenizer", author, version, about = "Neuratal Tokenizer (NTokenizer) — multilingual byte-level BPE tokenizer", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Inspect a tokenizer model or display base configuration
    Inspect {
        #[arg(short, long)]
        model: Option<String>,
    },
    /// Encode input text using the tokenizer
    Encode {
        #[arg(short, long)]
        text: String,
        #[arg(short, long)]
        model: Option<String>,
    },
    /// Decode token IDs back to text
    Decode {
        #[arg(short, long)]
        ids: String,
        #[arg(short, long)]
        model: Option<String>,
    },
    /// Train a new tokenizer model on a corpus file or manifest
    Train {
        /// Path to plain text or JSONL input file
        #[arg(short, long)]
        input: Option<PathBuf>,
        /// Path to JSON manifest file
        #[arg(short, long)]
        manifest: Option<PathBuf>,
        /// Output .ntok model file path
        #[arg(short, long, default_value = "tokenizer.ntok")]
        output: PathBuf,
        /// Desired target vocabulary size
        #[arg(short, long, default_value_t = 16384)]
        vocab_size: usize,
        /// Minimum pair frequency cutoff
        #[arg(long, default_value_t = 2)]
        min_freq: usize,
        /// Minimum corpus frequency for a unique pre-token to enter the pair index
        /// (1 keeps every pre-token; higher values reduce trainer memory)
        #[arg(long, default_value_t = 1)]
        min_word_freq: usize,
        /// Optional lower frequency floor applied only to pre-tokens composed of
        /// CJK (Han/Hiragana/Katakana/Hangul) characters; script-aware protection
        /// against pruning all pairs inside unsegmented CJK runs
        #[arg(long)]
        min_word_freq_cjk: Option<usize>,
        /// Pretokenization strategy (raw, conservative, script-digit, lws, script-transition, hybrid, code, math, cjk2, cjk4, cjk8, cjkhan4)
        #[arg(long, default_value = "conservative")]
        pretokenizer: String,
        /// Random seed for sampling and tie-breaking
        #[arg(long, default_value_t = 42)]
        seed: u64,
        /// Suppress periodic progress logging (progress is printed to stderr by default)
        #[arg(long)]
        quiet: bool,
    },
    /// Run empirical comparative benchmark across all pre-tokenization candidates
    Benchmark {
        /// Vocabulary budget for the comparative evaluation models
        #[arg(short, long, default_value_t = 450)]
        vocab_size: usize,
    },
    /// Validate a tokenizer model file: check integrity, print statistics
    Validate {
        /// Path to the .ntok model file to validate
        #[arg(short, long)]
        model: String,
        /// Run a quick encode/decode round-trip verification
        #[arg(long)]
        roundtrip: bool,
    },
    /// Migrate a legacy v1 (.rzt, pre-0.2.0) model file to a v2 (.ntok) file
    Migrate {
        /// Path to the legacy model file to read
        #[arg(short, long)]
        input: PathBuf,
        /// Path of the v2 model file to write
        #[arg(short, long, default_value = "tokenizer.ntok")]
        output: PathBuf,
    },
    /// Export a smaller-vocabulary model from a trained model's learned merge order
    Export {
        /// Input .ntok model file to trim
        #[arg(short, long)]
        model: PathBuf,
        /// Target vocabulary size for a single export (mutually exclusive with --sizes)
        #[arg(long)]
        vocab_size: Option<usize>,
        /// Comma-separated target sizes to export at once (e.g. 8192,16384,32768,65536,131072,196608,262144)
        #[arg(long, value_delimiter = ',')]
        sizes: Vec<usize>,
        /// Output file (with --vocab-size); defaults to <model-stem>-<vocab_size>.ntok next to the input
        #[arg(long)]
        output: Option<PathBuf>,
        /// Output directory (with --sizes); defaults to the input file's parent directory
        #[arg(long)]
        output_dir: Option<PathBuf>,
    },
}

fn load_optional_model(model: &Option<String>) -> Result<Tokenizer, String> {
    match model {
        Some(p) => load_ntok_file(p).map_err(|e| format!("Failed to load model file {}: {}", p, e)),
        None => Ok(Tokenizer::new_base()),
    }
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Inspect { model } => {
            println!("NTokenizer v{}", env!("CARGO_PKG_VERSION"));
            if let Some(path) = model {
                match load_ntok_file(&path) {
                    Ok(tok) => {
                        println!("Model file: {}", path);
                        println!("Total vocabulary size: {}", tok.vocab_size());
                        println!("Base byte tokens: 256");
                        println!("Learned merges: {}", tok.merges().len());
                        println!("Special tokens: {}", tok.special_tokens().len());
                        println!("Pretokenizer: {:?}", tok.pretokenizer());
                        println!("Normalization: {:?}", tok.normalization());
                    }
                    Err(e) => {
                        eprintln!("Failed to load model file: {}", e);
                        std::process::exit(1);
                    }
                }
            } else {
                let tok = Tokenizer::new_base();
                println!(
                    "Default base tokenizer vocabulary size: {}",
                    tok.vocab_size()
                );
                println!("Base byte tokens: 256");
                println!("Pretokenizer default: {:?}", tok.pretokenizer());
                println!("Normalization default: {:?}", tok.normalization());
            }
        }
        Commands::Encode { text, model } => {
            let tok = match load_optional_model(&model) {
                Ok(t) => t,
                Err(e) => {
                    eprintln!("{}", e);
                    std::process::exit(1);
                }
            };
            match tok.encode_text(&text) {
                Ok(tokens) => {
                    println!("Encoded {} bytes into {} tokens:", text.len(), tokens.len());
                    println!("{:?}", tokens);
                }
                Err(e) => eprintln!("Encoding error: {}", e),
            }
        }
        Commands::Decode { ids, model } => {
            let tok = match load_optional_model(&model) {
                Ok(t) => t,
                Err(e) => {
                    eprintln!("{}", e);
                    std::process::exit(1);
                }
            };
            let parsed_ids: Result<Vec<u32>, _> =
                ids.split_whitespace().map(|s| s.parse::<u32>()).collect();

            match parsed_ids {
                Ok(token_vec) => match tok.decode_text(&token_vec) {
                    Ok(text) => println!("Decoded text: {}", text),
                    Err(e) => eprintln!("Decoding error: {}", e),
                },
                Err(e) => eprintln!("Failed to parse token IDs: {}", e),
            }
        }
        Commands::Train {
            input,
            manifest,
            output,
            vocab_size,
            min_freq,
            min_word_freq,
            min_word_freq_cjk,
            pretokenizer,
            seed,
            quiet,
        } => {
            let manifest_path = manifest.clone();
            let input_path = input.clone();
            let pretok_mode = match pretokenizer.to_lowercase().as_str() {
                "raw" => PretokenizationMode::RawByte,
                "conservative" => PretokenizationMode::ConservativeUnicode,
                "script-digit" | "script" | "scriptaware" | "script_digit" => {
                    PretokenizationMode::ScriptAndDigitAware
                }
                "lws" | "leading-ws" | "leadingwhitespace" | "leading-whitespace-unicode" => {
                    PretokenizationMode::LeadingWhitespaceUnicode
                }
                "script-transition" | "scripttransition" | "transition" => {
                    PretokenizationMode::ScriptTransitionAware
                }
                "hybrid" | "unicode-hybrid" | "unicodehybrid" => {
                    PretokenizationMode::UnicodeHybrid
                }
                "code" | "code-aware" | "codeaware" => PretokenizationMode::CodeAware,
                "math" | "math-symbol" | "mathsymbol" | "math-symbol-aware" => {
                    PretokenizationMode::MathSymbolAware
                }
                "mathcjkhan4" | "cjkhan4" | "han" | "math-symbol-aware-cjk-han4" => {
                    PretokenizationMode::MathSymbolAwareCjkHan4
                }
                "mathcjk2" | "cjk2" | "math-symbol-aware-cjk2" => {
                    PretokenizationMode::MathSymbolAwareCjk2
                }
                "mathcjk4" | "cjk4" | "math-symbol-aware-cjk4" => {
                    PretokenizationMode::MathSymbolAwareCjk4
                }
                "mathcjk8" | "cjk8" | "math-symbol-aware-cjk8" => {
                    PretokenizationMode::MathSymbolAwareCjk8
                }
                other => {
                    eprintln!(
                        "Unknown pretokenizer mode '{}': expected 'raw', 'conservative', 'script-digit', 'lws', 'script-transition', 'hybrid', 'code', 'math', 'cjk2', 'cjk4', 'cjk8', or 'cjkhan4'",
                        other
                    );
                    std::process::exit(1);
                }
            };

            let manifest_obj = if let Some(m_path) = manifest {
                let content = std::fs::read_to_string(&m_path).unwrap_or_else(|e| {
                    eprintln!("Failed to read manifest {}: {}", m_path.display(), e);
                    std::process::exit(1);
                });
                serde_json::from_str(&content).unwrap_or_else(|e| {
                    eprintln!("Failed to parse manifest JSON {}: {}", m_path.display(), e);
                    std::process::exit(1);
                })
            } else if let Some(in_path) = input {
                let mut m = CorpusManifest::new(seed);
                m.add_entry(CorpusManifestEntry {
                    path: in_path,
                    weight: 1.0,
                    language: None,
                    domain: None,
                    text_field: None,
                });
                m
            } else {
                eprintln!("Error: Either --input or --manifest must be provided for training.");
                std::process::exit(1);
            };

            let config = TrainingConfig {
                target_vocab_size: vocab_size,
                min_frequency: min_freq,
                pretokenizer: pretok_mode,
                normalization: Normalization::None,
                special_tokens: vec![
                    "<|bos|>".to_string(),
                    "<|eos|>".to_string(),
                    "<|pad|>".to_string(),
                    "<|unk|>".to_string(),
                    "<|system|>".to_string(),
                    "<|user|>".to_string(),
                    "<|assistant|>".to_string(),
                ],
                seed,
            };

            // v0.2: capture provenance for the embedded model metadata.
            let train_started = utc_now_iso8601();
            let manifest_sha256 = manifest_path
                .as_deref()
                .map(file_sha256_hex)
                .unwrap_or_default();
            let input_sha256 = input_path
                .as_deref()
                .map(file_sha256_hex)
                .unwrap_or_default();
            let corpus_sha256 = if !manifest_sha256.is_empty() {
                manifest_sha256
            } else {
                input_sha256
            };
            let training_cli = format!(
                "ntokenizer train --manifest {} --vocab-size {} --min-freq {} --min-word-freq {}{} --pretokenizer {} --seed {} --output {}",
                manifest_path.as_deref().map(|p| p.display().to_string()).unwrap_or_else(|| "<single-file-input>".to_string()),
                vocab_size, min_freq, min_word_freq,
                min_word_freq_cjk.map(|f| format!(" --min-word-freq-cjk {f}")).unwrap_or_default(),
                pretokenizer, seed, output.display()
            );

            println!("Starting NTokenizer BPE training...");
            println!("Target vocabulary size: {}", config.target_vocab_size);
            println!("Pretokenization mode: {:?}", config.pretokenizer);

            match train_from_manifest_with_progress_and_script_aware_min_word_freq(
                &manifest_obj,
                &config,
                !quiet,
                min_word_freq,
                min_word_freq_cjk,
            ) {
                Ok((mut trained_tok, stats)) => {
                    println!("Training completed in {} ms!", stats.elapsed_millis);
                    println!("Records read: {}", stats.total_records_read);
                    println!("Bytes processed: {}", stats.total_bytes_processed);
                    println!("Unique words: {}", stats.unique_words_count);
                    if stats.pruned_words_count > 0 {
                        println!(
                            "Pruned words below min-word-freq: {} ({} used for training)",
                            stats.pruned_words_count, stats.training_words_count
                        );
                    }
                    println!("Merges learned: {}", stats.merges_learned);
                    println!("Final vocabulary size: {}", stats.final_vocab_size);

                    if stats.malformed_records_count > 0 {
                        println!(
                            "Warning: Skipped {} malformed records.",
                            stats.malformed_records_count
                        );
                    }

                    let mut metadata = ModelMetadata::default();
                    metadata.model_name = output
                        .file_stem()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_else(|| "ntokenizer".to_string());
                    metadata.model_version = env!("CARGO_PKG_VERSION").to_string();
                    metadata.format_version = neuratal_tokenizer::FORMAT_VERSION;
                    metadata.vocab_size = trained_tok.vocab_size() as u32;
                    metadata.base_byte_tokens = 256;
                    metadata.merges_count = trained_tok.merges().len() as u32;
                    metadata.special_tokens_count = trained_tok.special_tokens().len() as u32;
                    metadata.special_token_list = trained_tok.special_tokens().all_tokens().iter().map(|t| t.content.clone()).collect();
                    metadata.pretokenizer_mode = format!("{:?}", config.pretokenizer);
                    metadata.pretokenizer_version = "1".to_string();
                    metadata.normalization_mode = "None".to_string();
                    metadata.normalization = "None".to_string();
                    metadata.created_at = utc_now_iso8601();
                    metadata.training_started_at = train_started.clone();
                    metadata.training_finished_at = utc_now_iso8601();
                    metadata.min_frequency = min_freq as u32;
                    metadata.seed = seed;
                    metadata.corpus_manifest_sha256 = corpus_sha256;
                    metadata.corpus_bytes = stats.total_bytes_processed as u64;
                    metadata.corpus_records = stats.total_records_read as u64;
                    metadata.training_cli = training_cli;
                    metadata.licenses_reference = "data/v0.2/LICENSES.md".to_string();
                    metadata.git_commit = git_commit_hash();
                    trained_tok.set_metadata(metadata);

                    if let Err(e) = save_ntok_file(&trained_tok, &output) {
                        eprintln!("Failed to save output model to {:?}: {}", output, e);
                        std::process::exit(1);
                    }
                    println!("Saved model successfully to: {:?}", output);
                }
                Err(e) => {
                    eprintln!("Training failed: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Commands::Benchmark { vocab_size } => {
            println!(
                "Running empirical pre-tokenization benchmark (vocab budget: {})...",
                vocab_size
            );
            match neuratal_tokenizer::run_pretokenization_comparison(vocab_size) {
                Ok(report) => {
                    println!(
                        "\n========================================================================================="
                    );
                    println!(
                        "NEURATAL TOKENIZER EMPIRICAL PRE-TOKENIZATION BENCHMARK REPORT (VOCAB BUDGET: {})",
                        report.vocab_size
                    );
                    println!(
                        "=========================================================================================\n"
                    );

                    println!(
                        "{:<24} | {:<15} | {:<15} | {:<18} | {:<16}",
                        "Strategy",
                        "Overall Bytes/Tok",
                        "Overall Tok/Byte",
                        "Throughput (MB/s)",
                        "Tokens/sec"
                    );
                    println!(
                        "{:-<24}-+-{:-<15}-+-{:-<15}-+-{:-<18}-+-{:-<16}",
                        "", "", "", "", ""
                    );

                    for res in &report.results {
                        let strat_name = format!("{:?}", res.strategy);
                        println!(
                            "{:<24} | {:<15.3} | {:<15.3} | {:<18.2} | {:<16.0}",
                            strat_name,
                            res.overall_bytes_per_token,
                            res.overall_tokens_per_byte,
                            res.encoding_throughput_mb_s,
                            res.encoding_throughput_tokens_s,
                        );
                    }

                    println!(
                        "\n-----------------------------------------------------------------------------------------"
                    );
                    println!(
                        "DETAILED PER-CATEGORY COMPRESSION RATIO (BYTES PER TOKEN, HIGHER = BETTER COMPRESSION):"
                    );
                    println!(
                        "-----------------------------------------------------------------------------------------"
                    );

                    let categories = &report.results[0].category_metrics;
                    print!("{:<32}", "Category");
                    for res in &report.results {
                        print!(" | {:<18}", format!("{:?}", res.strategy));
                    }
                    println!();
                    print!("{:-<32}", "");
                    for _ in &report.results {
                        print!("-+{:-<20}", "");
                    }
                    println!();

                    for (i, cat) in categories.iter().enumerate() {
                        print!("{:<32}", cat.category_name);
                        for res in &report.results {
                            let bpt = res.category_metrics[i].bytes_per_token;
                            print!(" | {:<18.2}", bpt);
                        }
                        println!();
                    }
                }
                Err(e) => {
                    eprintln!("Benchmark failed: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Commands::Validate { model, roundtrip } => {
            let tok = match load_ntok_file(&model) {
                Ok(t) => t,
                Err(e) => {
                    eprintln!("Validation FAILED: {}", e);
                    std::process::exit(1);
                }
            };

            println!("Model file: {}", model);
            println!("File integrity (magic/format version/CRC32/payload length): OK");
            println!("Total vocabulary size: {}", tok.vocab_size());
            println!("Base byte tokens: 256");
            println!("Learned merges: {}", tok.merges().len());
            println!("Special tokens: {}", tok.special_tokens().len());
            println!(
                "Size breakdown: 256 base + {} merges + {} special tokens = {}",
                tok.merges().len(),
                tok.special_tokens().len(),
                256 + tok.merges().len() + tok.special_tokens().len()
            );
            println!("Pretokenizer: {:?}", tok.pretokenizer());
            println!("Normalization: {:?}", tok.normalization());

            let mut failed = false;
            let report = validate_tokenizer(&tok);
            for check in &report.checks {
                let status = if check.passed { "PASS" } else { "FAIL" };
                println!("[{}] {}: {}", status, check.name, check.detail);
                if !check.passed {
                    failed = true;
                }
            }

            // Optional extended multilingual round-trip probes
            if roundtrip {
                let probes = [
                    "English: The quick brown fox jumps over the lazy dog.",
                    "Türkçe: Şemsi Paşa Pasajı'nda sesi boğuk bir şoför gördüm.",
                    "Русский: Машинное обучение и многоязычные данные.",
                    "العربية: معالجة اللغة الطبيعية والتعلم الآلي.",
                    "עברית: למידת מכונה מודרנית.",
                    "中文：深度学习与自然语言处理。",
                    "日本語：大規模言語モデルのトークナイザー。",
                    "한국어: 자기회귀 언어 모델.",
                    "हिन्दी: प्राकृतिक भाषा प्रसंस्करण।",
                    "ไทย: แบบจำลองภาษาขนาดใหญ่",
                    "Emoji: 🚀 👨‍👩‍👧‍👦 🇹🇷 1️⃣",
                    "Math: ∫_0^1 x² dx = 1/3, ∑_{i=1}^n i = n(n+1)/2, e^{iπ} + 1 = 0",
                    "Code: fn main() { println!(\"hello, world\"); }",
                ];
                for probe in &probes {
                    let result = tok.encode_text(probe).and_then(|ids| tok.decode_text(&ids));
                    let short: String = probe.chars().take(40).collect();
                    match result {
                        Ok(decoded) => {
                            let ok = decoded == *probe;
                            println!(
                                "[{}] roundtrip probe: {}",
                                if ok { "PASS" } else { "FAIL" },
                                short
                            );
                            if !ok {
                                failed = true;
                            }
                        }
                        Err(e) => {
                            println!("[FAIL] roundtrip probe {}: {}", short, e);
                            failed = true;
                        }
                    }
                }
            }

            if failed {
                eprintln!("Validation FAILED for {}", model);
                std::process::exit(1);
            }
            println!("Validation PASSED for {}", model);
        }
        Commands::Migrate { input, output } => {
            // load_ntok_file accepts both (NTOK, 2) and legacy (RZT1, 1); saving
            // always writes the current v2 format.
            let tok = match load_ntok_file(&input) {
                Ok(t) => t,
                Err(e) => {
                    eprintln!(
                        "Migration failed: could not load {}: {}",
                        input.display(),
                        e
                    );
                    std::process::exit(1);
                }
            };
            if let Err(e) = save_ntok_file(&tok, &output) {
                eprintln!(
                    "Migration failed: could not write {}: {}",
                    output.display(),
                    e
                );
                std::process::exit(1);
            }
            println!(
                "Migrated {} -> {} (format version {} ('NTOK'), vocab size {})",
                input.display(),
                output.display(),
                neuratal_tokenizer::FORMAT_VERSION,
                tok.vocab_size()
            );
        }
        Commands::Export {
            model,
            vocab_size,
            sizes,
            output,
            output_dir,
        } => {
            if vocab_size.is_some() == !sizes.is_empty() {
                eprintln!("Error: exactly one of --vocab-size or --sizes must be provided.");
                std::process::exit(1);
            }
            if vocab_size.is_some() && output_dir.is_some() {
                eprintln!("Error: --output-dir is only valid with --sizes.");
                std::process::exit(1);
            }

            let main_model = match load_ntok_file(&model) {
                Ok(t) => t,
                Err(e) => {
                    eprintln!("Failed to load model file {}: {}", model.display(), e);
                    std::process::exit(1);
                }
            };
            let stem = model
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("exported")
                .to_string();
            let parent = model
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| PathBuf::from("."));

            let targets: Vec<usize> = if let Some(size) = vocab_size {
                vec![size]
            } else {
                sizes
            };

            for target in targets {
                let out_path = if vocab_size.is_some() {
                    output
                        .clone()
                        .unwrap_or_else(|| parent.join(format!("{}-{}.ntok", stem, target)))
                } else {
                    output_dir
                        .clone()
                        .unwrap_or_else(|| parent.clone())
                        .join(format!("{}-{}.ntok", stem, target))
                };
                // Ensure the output directory exists for --sizes exports.
                if let Some(dir) = out_path.parent()
                    && !dir.as_os_str().is_empty()
                {
                    let _ = std::fs::create_dir_all(dir);
                }

                let trimmed = match main_model.export_trimmed(target) {
                    Ok(t) => t,
                    Err(e) => {
                        eprintln!("Export to vocab size {} failed: {}", target, e);
                        std::process::exit(1);
                    }
                };

                if let Err(e) = save_ntok_file(&trimmed, &out_path) {
                    eprintln!("Failed to save {}: {}", out_path.display(), e);
                    std::process::exit(1);
                }

                // Sanity: the exported model must pass full validation.
                let report = validate_tokenizer(&trimmed);
                let failed = report.failures();
                if !failed.is_empty() {
                    for f in &failed {
                        eprintln!("[FAIL] export check {}: {}", f.name, f.detail);
                    }
                    eprintln!(
                        "Export to vocab size {} produced an invalid model; refusing to keep {}",
                        target,
                        out_path.display()
                    );
                    let _ = std::fs::remove_file(&out_path);
                    std::process::exit(1);
                }

                println!(
                    "Exported vocab {} -> {} ({} merges kept, {} special tokens, {} bytes)",
                    trimmed.vocab_size(),
                    out_path.display(),
                    trimmed.merges().len(),
                    trimmed.special_tokens().len(),
                    std::fs::metadata(&out_path).map(|m| m.len()).unwrap_or(0)
                );
            }
        }
    }
}
