//! Export a native `.ntok` model to a Hugging Face `tokenizer.json` (ByteLevel BPE).
//!
//! This performs a *structural* export: every NTokenizer byte sequence is mapped
//! to the GPT-2 byte-level Unicode alphabet and every learned merge is emitted in
//! rank order. Hugging Face's stock `ByteLevel` pre-tokenizer cannot reproduce
//! NTokenizer's custom script/punctuation/context pre-tokenization exactly, so
//! token-for-token parity on arbitrary text is **not** guaranteed. Decoding of
//! exported IDs is exact because the vocabulary is byte-complete.
//!
//! Example:
//!   cargo run --release --example export_hf -- \
//!     --model output/v0.3/ntokenizer-0.3-256k.ntok \
//!     --out output/v0.3/hf/tokenizer.json

use std::collections::BTreeMap;
use std::path::PathBuf;

use neuratal_tokenizer::{TokenId, load_ntok_file};
use serde_json::{Value, json};

/// GPT-2 byte -> Unicode character alphabet.
fn byte_to_unicode() -> [char; 256] {
    let mut bs: Vec<u16> = Vec::with_capacity(256);
    bs.extend(33u16..=126);
    bs.extend(161u16..=172);
    bs.extend(174u16..=255);
    let mut cs: Vec<u16> = bs.clone();
    let mut n: u16 = 0;
    for b in 0u16..=255 {
        if !bs.contains(&b) {
            bs.push(b);
            cs.push(256 + n);
            n += 1;
        }
    }
    let mut table = ['\0'; 256];
    for (&b, &c) in bs.iter().zip(cs.iter()) {
        table[b as usize] = char::from_u32(c as u32).expect("valid BMP scalar");
    }
    debug_assert!(table.iter().all(|&c| c != '\0'));
    table
}

fn token_string(bytes: &[u8], table: &[char; 256]) -> String {
    bytes.iter().map(|&b| table[b as usize]).collect()
}

fn main() {
    let mut model_path: Option<PathBuf> = None;
    let mut out_path = PathBuf::from("tokenizer.json");

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--model" | "-m" => model_path = args.next().map(PathBuf::from),
            "--out" | "-o" => out_path = args.next().map(PathBuf::from).unwrap_or(out_path),
            "--help" | "-h" => {
                println!("Usage: export_hf --model <model.ntok> [--out tokenizer.json]");
                return;
            }
            other => {
                eprintln!("Unknown argument: {other}");
                std::process::exit(2);
            }
        }
    }

    let model_path = match model_path {
        Some(p) => p,
        None => {
            eprintln!("--model is required");
            std::process::exit(2);
        }
    };

    let tokenizer = match load_ntok_file(&model_path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Failed to load {}: {e}", model_path.display());
            std::process::exit(1);
        }
    };

    let table = byte_to_unicode();
    let vocab_size = tokenizer.vocab_size();
    let specials = tokenizer.special_tokens();

    let mut vocab: BTreeMap<String, TokenId> = BTreeMap::new();
    let mut added_tokens: Vec<Value> = Vec::new();
    for token in specials.all_tokens() {
        added_tokens.push(json!({
            "id": token.id,
            "content": token.content,
            "single_word": false,
            "lstrip": token.lstrip,
            "rstrip": token.rstrip,
            "normalized": false,
            "special": true
        }));
    }

    for id in 0..vocab_size {
        let id = id as TokenId;
        let bytes = match tokenizer.vocab().get_bytes(id) {
            Some(b) => b,
            None => {
                eprintln!("Vocabulary is not dense: missing id {id}");
                std::process::exit(1);
            }
        };
        let key = if let Some(tok) = specials.all_tokens().iter().find(|t| t.id == id) {
            // Special tokens are emitted literally and registered as added tokens.
            tok.content.clone()
        } else {
            token_string(bytes, &table)
        };
        if let Some(previous) = vocab.insert(key.clone(), id) {
            eprintln!(
                "Duplicate token string {:?} for ids {previous} and {id}; cannot export losslessly",
                key
            );
            std::process::exit(1);
        }
    }

    // Validate and emit merges in rank order. Hugging Face infers the merged
    // token from the concatenation of the pair, so the pair must be contiguous
    // in NTokenizer's own vocabulary too.
    let rules = tokenizer.merges().rules();
    let mut merges: Vec<String> = Vec::with_capacity(rules.len());
    for rule in rules {
        let left = tokenizer
            .vocab()
            .get_bytes(rule.left)
            .expect("merge left id must exist");
        let right = tokenizer
            .vocab()
            .get_bytes(rule.right)
            .expect("merge right id must exist");
        let target = tokenizer
            .vocab()
            .get_bytes(rule.target)
            .expect("merge target id must exist");
        let mut expected = Vec::with_capacity(left.len() + right.len());
        expected.extend_from_slice(left);
        expected.extend_from_slice(right);
        if expected != target {
            eprintln!(
                "Merge rank {} ({}, {}) -> {} is not a contiguous byte concatenation; \
                 refusing to emit a tokenizer.json that would tokenize differently",
                rule.rank, rule.left, rule.right, rule.target
            );
            std::process::exit(1);
        }
        merges.push(format!(
            "{} {}",
            token_string(left, &table),
            token_string(right, &table)
        ));
    }

    let json = json!({
        "version": "1.0",
        "truncation": null,
        "padding": null,
        "added_tokens": added_tokens,
        "normalizer": null,
        "pre_tokenizer": {
            "type": "ByteLevel",
            "add_prefix_space": false,
            "trim_offsets": true,
            "use_regex": true
        },
        "post_processor": {
            "type": "ByteLevel",
            "add_prefix_space": true,
            "trim_offsets": false,
            "use_regex": true
        },
        "decoder": {
            "type": "ByteLevel",
            "add_prefix_space": false,
            "trim_offsets": true,
            "use_regex": true
        },
        "model": {
            "type": "BPE",
            "dropout": null,
            "unk_token": null,
            "continuing_subword_prefix": null,
            "end_of_word_suffix": null,
            "fuse_unk": false,
            "byte_fallback": false,
            "vocab": vocab,
            "merges": merges
        }
    });

    if let Some(parent) = out_path.parent() {
        if !parent.as_os_str().is_empty() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                eprintln!("Failed to create {}: {e}", parent.display());
                std::process::exit(1);
            }
        }
    }

    let encoded = match serde_json::to_vec(&json) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Failed to serialize tokenizer.json: {e}");
            std::process::exit(1);
        }
    };
    if let Err(e) = std::fs::write(&out_path, &encoded) {
        eprintln!("Failed to write {}: {e}", out_path.display());
        std::process::exit(1);
    }

    // Sidecar files so `AutoTokenizer.from_pretrained(<dir>)` resolves the
    // registered special tokens. They carry no model weights.
    let special_map: serde_json::Map<String, Value> = added_tokens
        .iter()
        .filter_map(|t| {
            let content = t.get("content")?.as_str()?;
            let key = match content {
                "<|bos|>" => Some("bos_token"),
                "<|eos|>" => Some("eos_token"),
                "<|pad|>" => Some("pad_token"),
                "<|unk|>" => Some("unk_token"),
                _ => None,
            }?;
            Some((key.to_string(), Value::String(content.to_string())))
        })
        .collect();
    if !special_map.is_empty() {
        let dir = out_path.parent().unwrap_or_else(|| std::path::Path::new("."));
        let special_path = dir.join("special_tokens_map.json");
        if let Ok(doc) = serde_json::to_vec_pretty(&special_map) {
            if let Err(e) = std::fs::write(&special_path, doc) {
                eprintln!("Failed to write {}: {e}", special_path.display());
            }
        }
        let config_name = "ntokenizer-hf-export".to_string();
        let mut config = serde_json::Map::new();
        config.insert("tokenizer_class".to_string(), Value::String("PreTrainedTokenizerFast".to_string()));
        config.insert("name_or_path".to_string(), Value::String(config_name));
        for key in ["bos_token", "eos_token", "pad_token", "unk_token"] {
            if let Some(v) = special_map.get(key) {
                config.insert(key.to_string(), v.clone());
            }
        }
        let config_path = dir.join("tokenizer_config.json");
        if let Ok(doc) = serde_json::to_vec_pretty(&config) {
            if let Err(e) = std::fs::write(&config_path, doc) {
                eprintln!("Failed to write {}: {e}", config_path.display());
            }
        }
    }

    let metadata = tokenizer.metadata();
    println!("Exported Hugging Face tokenizer.json");
    println!("  model:       {}", model_path.display());
    println!("  output:      {}", out_path.display());
    println!("  vocab:       {vocab_size} ids");
    println!("  merges:      {}", rules.len());
    println!("  added:       {} special tokens", specials.len());
    if let Some(m) = metadata {
        println!("  name:        {}", m.model_name);
        println!("  pretokenizer:{}", m.pretokenizer_mode);
    }
    println!();
    println!("WARNING: structural export only. HF ByteLevel pre-tokenization is not");
    println!("equivalent to NTokenizer's custom {:?} pre-tokenizer;", tokenizer.pretokenizer());
    println!("token-for-token parity on arbitrary text is not guaranteed.");
    println!("Byte-exact decode is preserved for exported token IDs.");
}
