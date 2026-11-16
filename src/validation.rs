//! Structural and semantic validation for Neuratal Tokenizer models.
//!
//! Used by the `validate` CLI command and by the deserializer to reject
//! corrupted or inconsistent model files. Validation never mutates the
//! tokenizer; it only inspects it.
//!
//! Note: the `.ntok` CRC32 checksum is an integrity check against accidental
//! corruption, not a cryptographic authentication mechanism.

use crate::Tokenizer;

/// A single validation check and its outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationCheck {
    /// Machine-readable check name.
    pub name: String,
    /// True when the check passed.
    pub passed: bool,
    /// Human-readable detail (explanation, counts, or failure reason).
    pub detail: String,
}

/// Aggregate report produced by [`validate_tokenizer`].
#[derive(Debug, Clone)]
pub struct ValidationReport {
    /// Individual checks, in execution order.
    pub checks: Vec<ValidationCheck>,
}

impl ValidationReport {
    /// Returns true when every check passed.
    pub fn is_ok(&self) -> bool {
        self.checks.iter().all(|c| c.passed)
    }

    /// Returns the names of all failed checks.
    pub fn failures(&self) -> Vec<&ValidationCheck> {
        self.checks.iter().filter(|c| !c.passed).collect()
    }
}

fn check(name: &str, result: Result<String, String>) -> ValidationCheck {
    match result {
        Ok(detail) => ValidationCheck {
            name: name.to_string(),
            passed: true,
            detail,
        },
        Err(detail) => ValidationCheck {
            name: name.to_string(),
            passed: false,
            detail,
        },
    }
}

/// Validates only structural/semantic consistency (no round-trip probes).
///
/// Intended for the deserializer and other integrity-critical paths.
/// Returns `Err` with a descriptive message on the first failed check.
pub fn validate_model_structure(tok: &Tokenizer) -> Result<(), String> {
    tok.vocab().validate()?;
    check_merges(tok)?;
    check_special_tokens(tok)?;
    // v3 payloads embed metadata: verify it matches the actual contents.
    if let Some(m) = tok.metadata() {
        if m.magic != "NTOK" {
            return Err(format!("embedded metadata magic is {:?}, expected \"NTOK\"", m.magic));
        }
        if m.vocab_size as usize != tok.vocab().len() {
            return Err(format!(
                "embedded metadata vocab_size {} does not match actual {}",
                m.vocab_size,
                tok.vocab().len()
            ));
        }
        if m.merges_count as usize != tok.merges().len() {
            return Err(format!(
                "embedded metadata merges_count {} does not match actual {}",
                m.merges_count,
                tok.merges().len()
            ));
        }
        if m.special_tokens_count as usize != tok.special_tokens.len() {
            return Err(format!(
                "embedded metadata special_tokens_count {} does not match actual {}",
                m.special_tokens_count,
                tok.special_tokens.len()
            ));
        }
        if m.base_byte_tokens != 256 {
            return Err(format!(
                "embedded metadata base_byte_tokens is {}, expected 256",
                m.base_byte_tokens
            ));
        }
    }
    Ok(())
}

/// Validates a tokenizer model structurally and semantically.
///
/// Checks performed:
/// 1. Vocabulary structural integrity (base bytes 0..=255 present and correct).
/// 2. Merge table consistency (contiguous ranks, unique targets, valid IDs,
///    merged byte sequences equal the concatenation of their inputs).
/// 3. Special token consistency (IDs >= 256, unique contents, unique IDs, and any
///    referenced vocabulary entry has bytes exactly equal to the special content).
/// 4. Encoder/decoder round-trip compatibility on fixed probe inputs.
pub fn validate_tokenizer(tok: &Tokenizer) -> ValidationReport {
    let mut checks = Vec::new();

    // 1. Vocabulary structural integrity
    checks.push(check(
        "vocabulary_structure",
        tok.vocab().validate().map(|_| {
            format!(
                "vocabulary is structurally valid ({} tokens)",
                tok.vocab().len()
            )
        }),
    ));

    // 2. Merge consistency
    checks.push(check("merge_consistency", check_merges(tok)));

    // 3. Special token consistency
    checks.push(check(
        "special_token_consistency",
        check_special_tokens(tok),
    ));

    // 4. Encoder/decoder compatibility probes
    checks.push(check("roundtrip_text", check_roundtrip_text(tok)));
    checks.push(check("roundtrip_bytes", check_roundtrip_bytes(tok)));

    // 5. Embedded metadata consistency (v3 payloads carry it; v1/v2 do not)
    checks.push(check("metadata_consistency", check_metadata(tok)));

    ValidationReport { checks }
}

/// Verifies the embedded `ModelMetadata` (if any) against the actual model
/// contents: magic, format version, vocab size, merge count, special-token
/// count, and base byte count. A model without embedded metadata passes with a
/// note (legacy v1/v2 files).
fn check_metadata(tok: &Tokenizer) -> Result<String, String> {
    match tok.metadata() {
        None => Ok("no embedded metadata (legacy v1/v2 model)".to_string()),
        Some(m) => {
            if m.magic != "NTOK" {
                return Err(format!("metadata magic is {:?}, expected \"NTOK\"", m.magic));
            }
            if m.vocab_size as usize != tok.vocab().len() {
                return Err(format!(
                    "metadata vocab_size {} does not match actual {}",
                    m.vocab_size,
                    tok.vocab().len()
                ));
            }
            if m.merges_count as usize != tok.merges().len() {
                return Err(format!(
                    "metadata merges_count {} does not match actual {}",
                    m.merges_count,
                    tok.merges().len()
                ));
            }
            if m.special_tokens_count as usize != tok.special_tokens.len() {
                return Err(format!(
                    "metadata special_tokens_count {} does not match actual {}",
                    m.special_tokens_count,
                    tok.special_tokens.len()
                ));
            }
            if m.base_byte_tokens != 256 {
                return Err(format!(
                    "metadata base_byte_tokens is {}, expected 256",
                    m.base_byte_tokens
                ));
            }
            Ok(format!(
                "embedded metadata consistent (model '{}', format v{})",
                m.model_name, m.format_version
            ))
        }
    }
}

fn check_merges(tok: &Tokenizer) -> Result<String, String> {
    let rules = tok.merges().rules();
    let vocab = tok.vocab();
    let vocab_len = vocab.len();

    // Note on duplicate (left, right) pairs: two rules sharing a pair must both
    // have target bytes == bytes(left) ++ bytes(right), forcing identical vocab
    // byte sequences (rejected by Vocabulary::validate) or identical targets
    // (rejected above) or non-contiguous ranks (rejected above). Vocabulary
    // byte-uniqueness is the primary guard there.

    let mut seen_targets = std::collections::HashSet::with_capacity(rules.len());

    for (position, rule) in rules.iter().enumerate() {
        if rule.rank as usize != position {
            return Err(format!(
                "merge rank gap: rule at position {} has rank {} (expected contiguous ranks)",
                position, rule.rank
            ));
        }
        if rule.target < 256 {
            return Err(format!(
                "merge target ID {} is below the reserved base-byte range (rank {})",
                rule.target, rule.rank
            ));
        }
        for (label, id) in [
            ("left", rule.left),
            ("right", rule.right),
            ("target", rule.target),
        ] {
            if id as usize >= vocab_len {
                return Err(format!(
                    "merge rank {} references {} token ID {} but vocabulary size is {}",
                    rule.rank, label, id, vocab_len
                ));
            }
        }
        if !seen_targets.insert(rule.target) {
            return Err(format!(
                "merge target ID {} is assigned more than once (rank {})",
                rule.target, rule.rank
            ));
        }

        // Merged byte sequence must equal left ++ right.
        let left = vocab.get_bytes(rule.left).expect("left ID checked above");
        let right = vocab.get_bytes(rule.right).expect("right ID checked above");
        let target = vocab
            .get_bytes(rule.target)
            .expect("target ID checked above");
        let mut expected = Vec::with_capacity(left.len() + right.len());
        expected.extend_from_slice(left);
        expected.extend_from_slice(right);
        if target != expected.as_slice() {
            return Err(format!(
                "merge rank {} target bytes do not equal left ++ right (ID {})",
                rule.rank, rule.target
            ));
        }
    }

    Ok(format!("{} merge rules consistent", rules.len()))
}

fn check_special_tokens(tok: &Tokenizer) -> Result<String, String> {
    let registry = tok.special_tokens();
    let vocab = tok.vocab();

    let mut seen_ids = std::collections::HashSet::new();
    let mut seen_contents = std::collections::HashSet::new();

    for st in registry.all_tokens() {
        if st.id < 256 {
            return Err(format!(
                "special token '{}' uses reserved base-byte ID {}",
                st.content, st.id
            ));
        }
        if !seen_ids.insert(st.id) {
            return Err(format!(
                "special token ID {} is assigned more than once",
                st.id
            ));
        }
        if !seen_contents.insert(st.content.clone()) {
            return Err(format!(
                "special token '{}' is registered more than once",
                st.content
            ));
        }
        // A special token ID may reference either no vocabulary entry (decode uses
        // the registry) or a vocabulary entry whose bytes exactly equal the special
        // token's content (e.g. the literal string was trained into the model and
        // the special token reuses that entry). Anything else is corruption.
        if let Some(bytes) = vocab.get_bytes(st.id)
            && bytes != st.content.as_bytes()
        {
            return Err(format!(
                "special token '{}' ID {} points to vocabulary bytes that do not match its content ({:?})",
                st.content, st.id, bytes
            ));
        }
    }

    Ok(format!("{} special tokens consistent", registry.len()))
}

fn check_roundtrip_text(tok: &Tokenizer) -> Result<String, String> {
    let probe = "Hello, world! 🚀 Türkçe metin: 123 + עברית — ∑∫√";
    let encoded = tok
        .encode_text(probe)
        .map_err(|e| format!("encode failed: {}", e))?;
    let decoded = tok
        .decode_text(&encoded)
        .map_err(|e| format!("decode failed: {}", e))?;
    if decoded != probe {
        return Err("text round-trip mismatch on probe input".to_string());
    }
    Ok("text round-trip verified".to_string())
}

fn check_roundtrip_bytes(tok: &Tokenizer) -> Result<String, String> {
    // Probe with invalid UTF-8 and NUL bytes through the binary API.
    let probe: &[u8] = &[0x00, 0xFF, 0xFE, b'a', 0x80, 0x7F, b'\n'];
    let bytes = tok.encode_bytes(probe);
    let decoded = tok
        .decode_bytes(&bytes)
        .map_err(|e| format!("byte decode failed: {}", e))?;
    if decoded != probe {
        return Err("byte round-trip mismatch on probe input".to_string());
    }
    Ok("byte round-trip verified".to_string())
}
