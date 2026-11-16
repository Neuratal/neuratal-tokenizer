//! Serialization format specification, CRC32 integrity checks, and file I/O.
//!
//! Format v3 ("NTOK", version 3) is the current on-disk format. It carries the
//! same header layout and payload schema as v2 ("NTOK", version 2): magic (4) +
//! version (2) + CRC32 (4) + payload length (8) + JSON payload. The only
//! difference is that v3 payloads may embed a `metadata` object describing how
//! the model was produced (v2 payloads carry none and load as `metadata: None`).
//! Legacy v1 ("RZT1", version 1) files are also accepted. The loader accepts
//! v1/v2/v3; the writer always emits v3.

use crate::Tokenizer;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

/// 4-byte magic signature for Neuratal Tokenizer v2/v3 files: "NTOK" (0x4E 0x54 0x4F 0x4B).
pub const MAGIC_BYTES: &[u8; 4] = b"NTOK";

/// Legacy v1 magic signature ("RZT1"), still accepted on load for compatibility.
pub const MAGIC_BYTES_V1: &[u8; 4] = b"RZT1";

/// Current serialization format version (v3: v2 payload + embedded `metadata` object).
pub const FORMAT_VERSION: u16 = 3;

/// v3 serialization format version.
pub const FORMAT_VERSION_V3: u16 = 3;

/// v2 serialization format version (payload without embedded metadata).
pub const FORMAT_VERSION_V2: u16 = 2;

/// Legacy v1 serialization format version.
pub const FORMAT_VERSION_V1: u16 = 1;

/// Default model file extension (no dot), as used by the CLI.
pub const NTOK_EXTENSION: &str = "ntok";

/// Legacy model file extension (no dot), for reference and migration tooling.
pub const RZT_EXTENSION: &str = "rzt";

/// Header size in bytes: magic (4) + version (2) + CRC32 (4) + payload length (8).
/// Identical for v1 and v2, so the loader can dispatch on (magic, version) alone.
pub const HEADER_SIZE: usize = 18;

/// Maximum accepted payload size (1 GiB). Protection against malicious or
/// corrupt length fields forcing huge allocations. `.ntok` payloads for
/// realistic vocabularies are a few megabytes.
pub const MAX_PAYLOAD_BYTES: u64 = 1 << 30;

/// Standard metadata stored in model files.
///
/// v0.2 (format v3) embeds this object inside the `Tokenizer` payload under the
/// `metadata` key. The fields up to `created_at` existed in v0.1; the rest are
/// additive (serde-defaulted) so v0.1 payloads without them still load.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelMetadata {
    pub magic: String,
    pub format_version: u16,
    pub algorithm: String,
    pub vocab_size: u32,
    pub base_byte_tokens: u32,
    pub merges_count: u32,
    pub special_tokens_count: u32,
    pub pretokenizer_mode: String,
    pub normalization_mode: String,
    pub created_at: String,
    // --- v0.2 additive fields (serde-defaulted for backward compatibility) ---
    #[serde(default)]
    pub model_name: String,
    #[serde(default)]
    pub model_version: String,
    #[serde(default)]
    pub tokenizer_crate_version: String,
    #[serde(default)]
    pub training_tool: String,
    #[serde(default)]
    pub min_frequency: u32,
    #[serde(default)]
    pub seed: u64,
    #[serde(default)]
    pub special_token_list: Vec<String>,
    #[serde(default)]
    pub pretokenizer_version: String,
    #[serde(default)]
    pub normalization: String,
    #[serde(default)]
    pub corpus_manifest_sha256: String,
    #[serde(default)]
    pub corpus_bytes: u64,
    #[serde(default)]
    pub corpus_records: u64,
    #[serde(default)]
    pub training_started_at: String,
    #[serde(default)]
    pub training_finished_at: String,
    #[serde(default)]
    pub training_cli: String,
    #[serde(default)]
    pub licenses_reference: String,
    #[serde(default)]
    pub git_commit: String,
}

impl Default for ModelMetadata {
    fn default() -> Self {
        Self {
            magic: "NTOK".to_string(),
            format_version: FORMAT_VERSION,
            algorithm: "byte_level_bpe".to_string(),
            vocab_size: 256,
            base_byte_tokens: 256,
            merges_count: 0,
            special_tokens_count: 0,
            pretokenizer_mode: "ConservativeUnicode".to_string(),
            normalization_mode: "None".to_string(),
            created_at: "2026-11-14T00:00:00Z".to_string(),
            model_name: String::new(),
            model_version: String::new(),
            tokenizer_crate_version: env!("CARGO_PKG_VERSION").to_string(),
            training_tool: "neuratal-tokenizer (Rust)".to_string(),
            min_frequency: 2,
            seed: 42,
            special_token_list: Vec::new(),
            pretokenizer_version: String::new(),
            normalization: "None".to_string(),
            corpus_manifest_sha256: String::new(),
            corpus_bytes: 0,
            corpus_records: 0,
            training_started_at: String::new(),
            training_finished_at: String::new(),
            training_cli: String::new(),
            licenses_reference: String::new(),
            git_commit: String::new(),
        }
    }
}

impl ModelMetadata {
    /// Derives the per-model metadata that must always reflect the actual model
    /// contents (vocab size, merged token count, special tokens) and drops the
    /// run-specific timing/CLI fields that do not apply to trimmed exports.
    pub fn for_export(&self, model_name: &str, vocab_size: u32, merges_count: u32, special_tokens_count: u32) -> Self {
        let mut m = self.clone();
        m.model_name = model_name.to_string();
        m.vocab_size = vocab_size;
        m.merges_count = merges_count;
        m.special_tokens_count = special_tokens_count;
        m
    }
}

/// Computes an IEEE 802.3 CRC32 checksum in pure, safe Rust.
pub fn compute_crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &b in data {
        crc ^= b as u32;
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

/// Saves a Tokenizer to a binary `.ntok` file with magic header and CRC32 checksum.
///
/// Always writes format version 3 ("NTOK" magic): the payload is the Tokenizer
/// JSON, which embeds its `metadata` object when present (absent otherwise).
/// The file is written to a temporary sibling and atomically renamed into
/// place, so a crash mid-write cannot truncate an existing model file.
pub fn save_ntok_file<P: AsRef<Path>>(tokenizer: &Tokenizer, path: P) -> Result<(), String> {
    let payload = serde_json::to_vec(tokenizer).map_err(|e| e.to_string())?;
    let crc = compute_crc32(&payload);

    let path = path.as_ref();
    let tmp_path = path.with_extension(format!("ntok.tmp-{}", std::process::id()));

    let write_result = (|| -> Result<(), String> {
        let mut file = File::create(&tmp_path).map_err(|e| e.to_string())?;
        // 1. Magic bytes (4 bytes)
        file.write_all(MAGIC_BYTES).map_err(|e| e.to_string())?;
        // 2. Format version (2 bytes, LE)
        file.write_all(&FORMAT_VERSION.to_le_bytes())
            .map_err(|e| e.to_string())?;
        // 3. CRC32 checksum (4 bytes, LE)
        file.write_all(&crc.to_le_bytes())
            .map_err(|e| e.to_string())?;
        // 4. Payload length (8 bytes, LE)
        file.write_all(&(payload.len() as u64).to_le_bytes())
            .map_err(|e| e.to_string())?;
        // 5. Payload bytes
        file.write_all(&payload).map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
        Ok(())
    })();

    if let Err(e) = write_result {
        let _ = std::fs::remove_file(&tmp_path);
        return Err(format!("Failed to write temporary model file: {}", e));
    }

    std::fs::rename(&tmp_path, path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp_path);
        format!("Failed to rename temporary model file into place: {}", e)
    })
}

/// Loads and validates a Tokenizer from a binary `.ntok` file.
///
/// Accepts format version 3 ("NTOK", current), format version 2 ("NTOK",
/// payload without embedded `metadata`, read as `metadata: None`), and legacy
/// format version 1 ("RZT1"). All versions are held to the same strict
/// validation; only the files' declared format version (and magic) differ.
///
/// The loader is strict: any magic/version/length/CRC mismatch, any truncated
/// or oversized payload, any trailing garbage, or any structurally inconsistent
/// tokenizer payload is rejected with a descriptive error. Deserialization
/// never silently accepts corrupted model data.
pub fn load_ntok_file<P: AsRef<Path>>(path: P) -> Result<Tokenizer, String> {
    let path_ref = path.as_ref();
    let file_len = std::fs::metadata(path_ref)
        .map_err(|e| format!("Failed to stat model file {}: {}", path_ref.display(), e))?
        .len();

    if file_len < HEADER_SIZE as u64 {
        return Err(format!(
            "File too small to be an .ntok model: {} bytes (header requires {} bytes)",
            file_len, HEADER_SIZE
        ));
    }

    let mut file = File::open(path_ref).map_err(|e| e.to_string())?;

    // 1. Verify magic bytes (v2 "NTOK" or legacy v1 "RZT1")
    let mut magic = [0u8; 4];
    file.read_exact(&mut magic)
        .map_err(|e| format!("Failed to read magic bytes: {}", e))?;

    // 2. Read format version
    let mut version_bytes = [0u8; 2];
    file.read_exact(&mut version_bytes)
        .map_err(|e| format!("Failed to read format version: {}", e))?;
    let version = u16::from_le_bytes(version_bytes);

    let is_v3 = &magic == MAGIC_BYTES && version == FORMAT_VERSION;
    let is_v2 = &magic == MAGIC_BYTES && version == FORMAT_VERSION_V2;
    let is_v1 = &magic == MAGIC_BYTES_V1 && version == FORMAT_VERSION_V1;
    if !is_v3 && !is_v2 && !is_v1 {
        return Err(format!(
            "Unsupported file format: magic {:?}, version {}. Expected magic 'NTOK' with version {} (v3) or {} (v2), or legacy magic 'RZT1' with version {} (v1)",
            magic, version, FORMAT_VERSION, FORMAT_VERSION_V2, FORMAT_VERSION_V1
        ));
    }

    // 3. Read CRC32
    let mut crc_bytes = [0u8; 4];
    file.read_exact(&mut crc_bytes)
        .map_err(|e| format!("Failed to read CRC32 checksum: {}", e))?;
    let expected_crc = u32::from_le_bytes(crc_bytes);

    // 4. Read payload length
    let mut len_bytes = [0u8; 8];
    file.read_exact(&mut len_bytes)
        .map_err(|e| format!("Failed to read payload length: {}", e))?;
    let payload_len = u64::from_le_bytes(len_bytes);

    // 4a. Guard against oversized payloads before allocating.
    if payload_len > MAX_PAYLOAD_BYTES {
        return Err(format!(
            "Payload length {} exceeds the maximum supported size of {} bytes",
            payload_len, MAX_PAYLOAD_BYTES
        ));
    }

    // 4b. The declared payload must exactly cover the rest of the file.
    // Any mismatch means a truncated, spliced, or padded file.
    let expected_total = (HEADER_SIZE as u64)
        .checked_add(payload_len)
        .ok_or_else(|| "Payload length overflows the file size computation".to_string())?;
    if expected_total != file_len {
        return Err(format!(
            "Payload length mismatch: header declares {} bytes of payload ({} total) but the file is {} bytes",
            payload_len, expected_total, file_len
        ));
    }

    // 5. Read payload (size is bounded by MAX_PAYLOAD_BYTES and verified against the file)
    let mut payload = vec![0u8; payload_len as usize];
    file.read_exact(&mut payload)
        .map_err(|e| format!("Failed to read payload data: {}", e))?;

    // 6. Verify checksum
    let actual_crc = compute_crc32(&payload);
    if actual_crc != expected_crc {
        return Err(format!(
            "File integrity check failed: CRC32 mismatch (expected 0x{:08X}, calculated 0x{:08X})",
            expected_crc, actual_crc
        ));
    }

    // 7. Deserialize tokenizer
    let tokenizer: Tokenizer = serde_json::from_slice(&payload)
        .map_err(|e| format!("Failed to deserialize tokenizer payload: {}", e))?;

    // 8. Full structural validation (vocabulary, merges, special tokens)
    crate::validation::validate_model_structure(&tokenizer)?;
    Ok(tokenizer)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crc32_standard_vectors() {
        let crc = compute_crc32(b"123456789");
        assert_eq!(crc, 0xCBF43926);

        let empty_crc = compute_crc32(b"");
        assert_eq!(empty_crc, 0x00000000);
    }

    #[test]
    fn test_save_and_load_ntok_binary() {
        let tok = Tokenizer::new_base();
        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("test_model.ntok");

        save_ntok_file(&tok, &file_path).unwrap();
        let loaded = load_ntok_file(&file_path).unwrap();

        assert_eq!(loaded.vocab_size(), tok.vocab_size());
        assert_eq!(loaded.pretokenizer(), tok.pretokenizer());

        // Verify the saved file is v2 ("NTOK" magic)
        let raw = std::fs::read(&file_path).unwrap();
        assert_eq!(&raw[..4], b"NTOK");
        assert_eq!(u16::from_le_bytes([raw[4], raw[5]]), FORMAT_VERSION);

        let _ = std::fs::remove_file(file_path);
    }

    /// Builds a legacy v1 ("RZT1") file from a tokenizer, by hand, so the
    /// compat loader can be exercised against real v1 bytes.
    fn make_v1_file_bytes(tok: &Tokenizer) -> Vec<u8> {
        let payload = serde_json::to_vec(tok).unwrap();
        let crc = compute_crc32(&payload);

        let mut out = Vec::with_capacity(HEADER_SIZE + payload.len());
        out.extend_from_slice(MAGIC_BYTES_V1);
        out.extend_from_slice(&FORMAT_VERSION_V1.to_le_bytes());
        out.extend_from_slice(&crc.to_le_bytes());
        out.extend_from_slice(&(payload.len() as u64).to_le_bytes());
        out.extend_from_slice(&payload);
        out
    }

    #[test]
    fn test_load_legacy_v1_file() {
        let tok = Tokenizer::new_base();
        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("legacy_v1_model.rzt");

        std::fs::write(&file_path, make_v1_file_bytes(&tok)).unwrap();
        let loaded = load_ntok_file(&file_path).expect("legacy v1 file must load");
        assert_eq!(loaded.vocab_size(), tok.vocab_size());
        assert_eq!(loaded.merges().len(), tok.merges().len());
        assert_eq!(loaded.merges().rules(), tok.merges().rules());

        // Re-saving produces a v2 file that round-trips identically
        let v2_path = temp_dir.join("legacy_migrated_model.ntok");
        save_ntok_file(&loaded, &v2_path).unwrap();
        let raw = std::fs::read(&v2_path).unwrap();
        assert_eq!(&raw[..4], b"NTOK");
        let reloaded = load_ntok_file(&v2_path).unwrap();
        assert_eq!(reloaded.vocab_size(), tok.vocab_size());

        let _ = std::fs::remove_file(file_path);
        let _ = std::fs::remove_file(v2_path);
    }

    #[test]
    fn test_legacy_v1_file_with_corrupt_crc_rejected() {
        let tok = Tokenizer::new_base();
        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("legacy_v1_corrupt.rzt");

        let mut bytes = make_v1_file_bytes(&tok);
        bytes[6] ^= 0x01; // flip one bit in the stored CRC32
        std::fs::write(&file_path, &bytes).unwrap();

        let err = load_ntok_file(&file_path).unwrap_err();
        assert!(err.contains("CRC32 mismatch"), "unexpected error: {}", err);
        let _ = std::fs::remove_file(file_path);
    }

    #[test]
    fn test_v1_magic_with_v2_version_rejected() {
        // Mixed magic/version must not be accepted: only (NTOK, 2) and (RZT1, 1).
        let tok = Tokenizer::new_base();
        let temp_dir = std::env::temp_dir();

        for (tag, magic, version) in [
            ("v1magic_v2ver", MAGIC_BYTES_V1, FORMAT_VERSION),
            ("v2magic_v1ver", MAGIC_BYTES, FORMAT_VERSION_V1),
            ("unknown_magic", b"ZZZZ", FORMAT_VERSION),
        ] {
            let mut bytes = make_v1_file_bytes(&tok);
            bytes[..4].copy_from_slice(magic);
            bytes[4..6].copy_from_slice(&version.to_le_bytes());
            let path = temp_dir.join(format!("{}.ntok", tag));
            std::fs::write(&path, &bytes).unwrap();
            let err = load_ntok_file(&path).unwrap_err();
            assert!(
                err.contains("Unsupported file format"),
                "{} must be rejected, got: {}",
                tag,
                err
            );
            let _ = std::fs::remove_file(path);
        }
    }

    #[test]
    fn test_corrupted_ntok_files() {
        let tok = Tokenizer::new_base();
        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("test_corrupted_model.ntok");

        save_ntok_file(&tok, &file_path).unwrap();
        let original_bytes = std::fs::read(&file_path).unwrap();

        // 1. Corrupted Magic
        let mut bad_magic = original_bytes.clone();
        bad_magic[0] = b'X';
        let bad_magic_path = temp_dir.join("bad_magic.ntok");
        std::fs::write(&bad_magic_path, &bad_magic).unwrap();
        let err = load_ntok_file(&bad_magic_path).unwrap_err();
        assert!(err.contains("Unsupported file format"));
        let _ = std::fs::remove_file(bad_magic_path);

        // 2. Unsupported Version (set version to 99)
        let mut bad_version = original_bytes.clone();
        bad_version[4] = 99;
        bad_version[5] = 0;
        let bad_ver_path = temp_dir.join("bad_version.ntok");
        std::fs::write(&bad_ver_path, &bad_version).unwrap();
        let err = load_ntok_file(&bad_ver_path).unwrap_err();
        assert!(err.contains("Unsupported file format"));
        let _ = std::fs::remove_file(bad_ver_path);

        // 3. Bit-flipped Payload (CRC32 Mismatch)
        let mut bit_flip = original_bytes.clone();
        let last_idx = bit_flip.len() - 1;
        bit_flip[last_idx] ^= 0xFF; // Flip bits
        let bit_flip_path = temp_dir.join("bit_flip.ntok");
        std::fs::write(&bit_flip_path, &bit_flip).unwrap();
        let err = load_ntok_file(&bit_flip_path).unwrap_err();
        assert!(err.contains("File integrity check failed: CRC32 mismatch"));
        let _ = std::fs::remove_file(bit_flip_path);

        // 4. Truncated File (fewer than the full header)
        let truncated = original_bytes[..10].to_vec();
        let trunc_path = temp_dir.join("truncated.ntok");
        std::fs::write(&trunc_path, &truncated).unwrap();
        let err = load_ntok_file(&trunc_path).unwrap_err();
        assert!(err.contains("File too small"));
        let _ = std::fs::remove_file(trunc_path);

        // 5. Empty File
        let empty_path = temp_dir.join("empty.ntok");
        std::fs::write(&empty_path, b"").unwrap();
        let err = load_ntok_file(&empty_path).unwrap_err();
        assert!(err.contains("File too small"));
        let _ = std::fs::remove_file(empty_path);

        let _ = std::fs::remove_file(file_path);
    }
}
