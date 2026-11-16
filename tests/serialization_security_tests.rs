//! Phase 5C/5F adversarial serialization tests for the `.ntok` format.
//!
//! Every malformed file produced here must be *rejected* with a descriptive
//! error. Deserialization must never silently accept corrupted model data.

use neuratal_tokenizer::{
    CorpusManifest, CorpusManifestEntry, FORMAT_VERSION, MAGIC_BYTES, MAX_PAYLOAD_BYTES,
    Normalization, PretokenizationMode, Tokenizer, TrainingConfig, compute_crc32, load_ntok_file,
    save_ntok_file, train_from_manifest,
};
use std::io::Write as _;
use std::path::PathBuf;

static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

fn temp_path(tag: &str) -> PathBuf {
    let id = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    std::env::temp_dir().join(format!(
        "ntok_adversarial_{}_{}_{}.ntok",
        tag,
        std::process::id(),
        id
    ))
}

fn make_valid_model_bytes() -> Vec<u8> {
    let tok = Tokenizer::new_base();
    let path = temp_path("valid_source");
    save_ntok_file(&tok, &path).unwrap();
    let bytes = std::fs::read(&path).unwrap();
    let _ = std::fs::remove_file(&path);
    bytes
}

fn make_trained_model_bytes() -> Vec<u8> {
    let corpus_path = temp_path("adversarial_corpus").with_extension("txt");
    {
        let mut f = std::fs::File::create(&corpus_path).unwrap();
        for _ in 0..8 {
            writeln!(f, "the quick brown fox jumps over the lazy dog").unwrap();
            writeln!(f, "the cat sat on the mat with the rat").unwrap();
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
        target_vocab_size: 274, // 256 base + 16 merges + 2 special
        min_frequency: 2,
        pretokenizer: PretokenizationMode::RawByte,
        normalization: Normalization::None,
        special_tokens: vec!["<|bos|>".to_string(), "<|eos|>".to_string()],
        seed: 42,
    };

    let (tok, _) = train_from_manifest(&manifest, &config).unwrap();
    let model_path = temp_path("trained_source");
    save_ntok_file(&tok, &model_path).unwrap();
    let bytes = std::fs::read(&model_path).unwrap();
    let _ = std::fs::remove_file(model_path);
    let _ = std::fs::remove_file(corpus_path);
    bytes
}

fn write_bytes(tag: &str, bytes: &[u8]) -> PathBuf {
    let path = temp_path(tag);
    std::fs::write(&path, bytes).unwrap();
    path
}

#[test]
fn rejects_payload_length_above_cap() {
    let mut bytes = make_valid_model_bytes();
    // Header: magic(4) + version(2) + crc(4) + len(8)
    let huge_len: u64 = MAX_PAYLOAD_BYTES + 1;
    bytes[10..18].copy_from_slice(&huge_len.to_le_bytes());

    let path = write_bytes("huge_len", &bytes);
    let err = load_ntok_file(&path).unwrap_err();
    assert!(
        err.contains("exceeds the maximum"),
        "unexpected error: {}",
        err
    );
    let _ = std::fs::remove_file(path);
}

#[test]
fn rejects_payload_length_u64_max() {
    let mut bytes = make_valid_model_bytes();
    bytes[10..18].copy_from_slice(&u64::MAX.to_le_bytes());

    let path = write_bytes("u64_max_len", &bytes);
    let err = load_ntok_file(&path).unwrap_err();
    assert!(
        err.contains("exceeds the maximum"),
        "unexpected error: {}",
        err
    );
    let _ = std::fs::remove_file(path);
}

#[test]
fn rejects_declared_length_mismatch() {
    let mut bytes = make_valid_model_bytes();
    let declared = u64::from_le_bytes(bytes[10..18].try_into().unwrap());
    bytes[10..18].copy_from_slice(&(declared + 1).to_le_bytes());

    let path = write_bytes("len_mismatch", &bytes);
    let err = load_ntok_file(&path).unwrap_err();
    assert!(
        err.contains("Payload length mismatch"),
        "unexpected error: {}",
        err
    );
    let _ = std::fs::remove_file(path);
}

#[test]
fn rejects_truncated_mid_payload() {
    let bytes = make_valid_model_bytes();
    let truncated = &bytes[..bytes.len() - 5];

    let path = write_bytes("mid_truncate", truncated);
    let err = load_ntok_file(&path).unwrap_err();
    assert!(
        err.contains("Payload length mismatch") || err.contains("File too small"),
        "unexpected error: {}",
        err
    );
    let _ = std::fs::remove_file(path);
}

#[test]
fn rejects_trailing_garbage() {
    let mut bytes = make_valid_model_bytes();
    bytes.extend_from_slice(&[0xDE, 0xAD, 0xBE, 0xEF]);

    let path = write_bytes("trailing", &bytes);
    let err = load_ntok_file(&path).unwrap_err();
    assert!(
        err.contains("Payload length mismatch"),
        "unexpected error: {}",
        err
    );
    let _ = std::fs::remove_file(path);
}

#[test]
fn rejects_crc_bit_flip() {
    let mut bytes = make_valid_model_bytes();
    bytes[6] ^= 0x01; // flip one bit in the stored CRC32

    let path = write_bytes("crc_flip", &bytes);
    let err = load_ntok_file(&path).unwrap_err();
    assert!(err.contains("CRC32 mismatch"), "unexpected error: {}", err);
    let _ = std::fs::remove_file(path);
}

#[test]
fn rejects_unsupported_version_on_v2_file() {
    // A v2 ("NTOK") file with a non-v2 version must be rejected. Note: version 2
    // is the current format, so it is not in this list; (RZT1 magic, version 2) is
    // covered in the format unit tests.
    for bad_version in [0u16, 1u16, 99u16, 257u16] {
        let mut bytes = make_valid_model_bytes();
        bytes[4..6].copy_from_slice(&bad_version.to_le_bytes());

        let path = write_bytes("bad_version", &bytes);
        let err = load_ntok_file(&path).unwrap_err();
        assert!(
            err.contains("Unsupported file format"),
            "unexpected error for version {}: {}",
            bad_version,
            err
        );
        let _ = std::fs::remove_file(path);
    }
}

/// Rewrites a valid trained model file with a mutated payload, re-computing
/// the CRC32 so the file passes integrity checks and reaches structural validation.
fn rewrite_payload_with_crc<F>(bytes: &[u8], tag: &str, mutate: F) -> PathBuf
where
    F: FnOnce(&mut serde_json::Value),
{
    let payload_len = u64::from_le_bytes(bytes[10..18].try_into().unwrap()) as usize;
    let payload = &bytes[18..18 + payload_len];
    let mut value: serde_json::Value = serde_json::from_slice(payload).unwrap();
    mutate(&mut value);
    let new_payload = serde_json::to_vec(&value).unwrap();

    let mut out = Vec::with_capacity(18 + new_payload.len());
    out.extend_from_slice(MAGIC_BYTES);
    out.extend_from_slice(&FORMAT_VERSION.to_le_bytes());
    out.extend_from_slice(&compute_crc32(&new_payload).to_le_bytes());
    out.extend_from_slice(&(new_payload.len() as u64).to_le_bytes());
    out.extend_from_slice(&new_payload);

    write_bytes(tag, &out)
}

#[test]
fn rejects_merge_target_out_of_vocab() {
    let bytes = make_trained_model_bytes();
    let path = rewrite_payload_with_crc(&bytes, "merge_target_oob", |v| {
        v["merges"]["rules"][0]["target"] = serde_json::json!(9_999_999u32);
    });

    let err = load_ntok_file(&path).unwrap_err();
    assert!(
        err.contains("vocabulary size") || err.contains("references target"),
        "unexpected error: {}",
        err
    );
    let _ = std::fs::remove_file(path);
}

#[test]
fn rejects_merge_rank_gap() {
    let bytes = make_trained_model_bytes();
    let path = rewrite_payload_with_crc(&bytes, "merge_rank_gap", |v| {
        v["merges"]["rules"][0]["rank"] = serde_json::json!(41u32);
    });

    let err = load_ntok_file(&path).unwrap_err();
    assert!(err.contains("rank gap"), "unexpected error: {}", err);
    let _ = std::fs::remove_file(path);
}

#[test]
fn rejects_merge_bytes_inconsistent_with_inputs() {
    let bytes = make_trained_model_bytes();
    let path = rewrite_payload_with_crc(&bytes, "merge_bytes_bad", |v| {
        // Corrupt the byte sequence of merge target 256 (first learned token).
        let id_to_bytes = v["vocab"]["id_to_bytes"].as_array_mut().unwrap();
        id_to_bytes[256] = serde_json::json!([0xFF, 0xFE, 0xFD]);
    });

    let err = load_ntok_file(&path).unwrap_err();
    assert!(
        err.contains("do not equal left ++ right"),
        "unexpected error: {}",
        err
    );
    let _ = std::fs::remove_file(path);
}

#[test]
fn rejects_special_token_pointing_at_wrong_bytes() {
    // A special token whose ID references a vocabulary entry whose bytes do not
    // equal the special token's content is corrupt (e.g. an ID-reordering bug).
    let bytes = make_trained_model_bytes();
    let path = rewrite_payload_with_crc(&bytes, "special_wrong_bytes", |v| {
        let merge_target = v["merges"]["rules"][0]["target"].as_u64().unwrap();
        v["special_tokens"]["tokens"][0]["id"] = serde_json::json!(merge_target);
    });

    let err = load_ntok_file(&path).unwrap_err();
    assert!(
        err.contains("do not match its content"),
        "unexpected error: {}",
        err
    );
    let _ = std::fs::remove_file(path);
}

#[test]
fn rejects_special_token_id_below_256() {
    let bytes = make_trained_model_bytes();
    let path = rewrite_payload_with_crc(&bytes, "special_low_id", |v| {
        v["special_tokens"]["tokens"][0]["id"] = serde_json::json!(17u32);
    });

    let err = load_ntok_file(&path).unwrap_err();
    assert!(
        err.contains("reserved base-byte ID"),
        "unexpected error: {}",
        err
    );
    let _ = std::fs::remove_file(path);
}

#[test]
fn rejects_garbage_payload_with_valid_crc() {
    // Hand-build a file whose CRC matches garbage bytes: integrity passes,
    // deserialization must still fail.
    let garbage = b"this is definitely not a serialized Tokenizer payload";
    let mut out = Vec::with_capacity(18 + garbage.len());
    out.extend_from_slice(MAGIC_BYTES);
    out.extend_from_slice(&FORMAT_VERSION.to_le_bytes());
    out.extend_from_slice(&compute_crc32(garbage).to_le_bytes());
    out.extend_from_slice(&(garbage.len() as u64).to_le_bytes());
    out.extend_from_slice(garbage);

    let path = write_bytes("garbage_payload", &out);
    let err = load_ntok_file(&path).unwrap_err();
    assert!(
        err.contains("Failed to deserialize tokenizer payload"),
        "unexpected error: {}",
        err
    );
    let _ = std::fs::remove_file(path);
}

#[test]
fn rejects_empty_payload_with_valid_crc() {
    // payload_len = 0, CRC of empty input = 0x00000000
    let mut out = Vec::with_capacity(18);
    out.extend_from_slice(MAGIC_BYTES);
    out.extend_from_slice(&FORMAT_VERSION.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&0u64.to_le_bytes());

    let path = write_bytes("empty_payload", &out);
    let err = load_ntok_file(&path).unwrap_err();
    assert!(
        err.contains("Failed to deserialize tokenizer payload"),
        "unexpected error: {}",
        err
    );
    let _ = std::fs::remove_file(path);
}

#[test]
fn valid_trained_model_still_loads() {
    // Guard against the loader being *overly* strict and rejecting good files.
    let corpus_path = temp_path("valid_roundtrip_corpus").with_extension("txt");
    {
        let mut f = std::fs::File::create(&corpus_path).unwrap();
        for _ in 0..4 {
            writeln!(
                f,
                "a sufficiently repetitive corpus for compression 12345 12345"
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
        target_vocab_size: 270,
        min_frequency: 2,
        pretokenizer: PretokenizationMode::ConservativeUnicode,
        normalization: Normalization::None,
        special_tokens: vec!["<|bos|>".to_string(), "<|eos|>".to_string()],
        seed: 42,
    };
    let (tok, _) = train_from_manifest(&manifest, &config).unwrap();

    let path = temp_path("valid_model");
    save_ntok_file(&tok, &path).unwrap();
    let loaded = load_ntok_file(&path).expect("valid model must load");
    assert_eq!(loaded.vocab_size(), tok.vocab_size());
    assert_eq!(loaded.merges().len(), tok.merges().len());

    let _ = std::fs::remove_file(path);
    let _ = std::fs::remove_file(corpus_path);
}
