//! BPE Training coordinator and execution engine.
//!
//! Integrates streaming corpus ingestion, weighted sampling, pre-token aggregation,
//! inverted-index pair counting, and deterministic tie-breaking merge generation.

use crate::Tokenizer;
use crate::core::merges::MergeTable;
use crate::core::special_tokens::SpecialTokenRegistry;
use crate::core::vocabulary::Vocabulary;
use crate::encoding::normalizer::Normalization;
use crate::encoding::pretokenizer::PretokenizationMode;
use crate::training::corpus::{CorpusFormat, CorpusRecordError, StreamingCorpusReader};
use crate::training::pair_counter::BpePairTrainer;
use crate::training::sampling::CorpusManifest;
use unicode_script::{Script as UnicodeScript, UnicodeScript as UnicodeScriptTrait};

/// Returns true when every script-bearing character in a pre-token is Han,
/// Hiragana, Katakana or Hangul. Common/Inherited characters (digits, marks,
/// punctuation shared with CJK) are allowed to co-occur. Used for script-aware
/// frequency floors: unsegmented CJK runs are long, unique pre-tokens whose
/// pair frequencies are individually rare, so a flat word-frequency floor
/// would silently discard every pair inside them.
fn pre_token_is_all_cjk(bytes: &[u8]) -> bool {
    match std::str::from_utf8(bytes) {
        Ok(s) => {
            let mut any = false;
            for ch in s.chars() {
                match ch.script() {
                    UnicodeScript::Han
                    | UnicodeScript::Hiragana
                    | UnicodeScript::Katakana
                    | UnicodeScript::Hangul => any = true,
                    UnicodeScript::Common
                    | UnicodeScript::Inherited
                    | UnicodeScript::Unknown => {}
                    _ => return false,
                }
            }
            any
        }
        Err(_) => false,
    }
}

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Training configuration for the BPE training engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingConfig {
    /// Desired total vocabulary size (including 256 base bytes and special tokens).
    pub target_vocab_size: usize,
    /// Minimum pair co-occurrence frequency to be eligible for merging.
    pub min_frequency: usize,
    /// Pre-tokenization strategy mode.
    pub pretokenizer: PretokenizationMode,
    /// Normalization strategy mode (default: None).
    pub normalization: Normalization,
    /// Special tokens to register in the trained tokenizer.
    pub special_tokens: Vec<String>,
    /// Random seed for deterministic sampling and tie-breaking.
    pub seed: u64,
}

impl Default for TrainingConfig {
    fn default() -> Self {
        Self {
            target_vocab_size: 16_384,
            min_frequency: 2,
            pretokenizer: PretokenizationMode::default(),
            normalization: Normalization::None,
            special_tokens: vec![
                "<|bos|>".to_string(),
                "<|eos|>".to_string(),
                "<|pad|>".to_string(),
                "<|unk|>".to_string(),
            ],
            seed: 42,
        }
    }
}

/// Statistics and diagnostic metrics from a completed training run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingStats {
    pub total_records_read: usize,
    pub total_bytes_processed: usize,
    pub malformed_records_count: usize,
    pub malformed_records_sample: Vec<String>,
    pub unique_words_count: usize,
    /// Unique pre-tokens removed because their corpus frequency was below the
    /// configured training floor. Zero for legacy (floor = 1) training.
    #[serde(default)]
    pub pruned_words_count: usize,
    /// Unique pre-tokens actually used to build the BPE pair index.
    #[serde(default)]
    pub training_words_count: usize,
    pub initial_pair_count: usize,
    pub initial_occurrences_count: usize,
    pub merges_learned: usize,
    pub final_vocab_size: usize,
    pub elapsed_millis: u128,
    /// Number of merge candidates dropped because their byte sequence already
    /// existed as a token (different segmentation of the same bytes).
    #[serde(default)]
    pub skipped_duplicate_merges: usize,
}

/// Trains a Neuratal Tokenizer model on a corpus manifest.
pub fn train_from_manifest(
    manifest: &CorpusManifest,
    config: &TrainingConfig,
) -> Result<(Tokenizer, TrainingStats), String> {
    train_from_manifest_with_progress(manifest, config, false)
}

/// Deterministic per-record sampling value in [0, 1), derived from the seed and the
/// record's global index. Unlike a sequential RNG stream, this does not depend on
/// processing order, so single-threaded and multithreaded ingestion produce
/// byte-identical results.
fn record_sample_value(seed: u64, record_index: u64) -> f64 {
    let mut z = seed ^ record_index.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    (z >> 11) as f64 * (1.0 / (1u64 << 53) as f64)
}

/// A record staged for parallel processing, with its deterministic global index.
struct IndexedRecord {
    global_index: u64,
    weight: f64,
    record: Result<crate::training::corpus::CorpusRecord, CorpusRecordError>,
}

/// Number of records buffered per parallel ingestion chunk.
const INGEST_CHUNK_SIZE: usize = 1024;

/// Accumulates per-chunk word counts and totals.
#[derive(Default)]
struct IngestAccumulator {
    word_counts: HashMap<Vec<u8>, usize>,
    total_bytes: usize,
    malformed_count: usize,
    malformed_samples: Vec<String>,
}

impl IngestAccumulator {
    fn merge(&mut self, other: IngestAccumulator) {
        for (k, v) in other.word_counts {
            *self.word_counts.entry(k).or_insert(0) += v;
        }
        self.total_bytes += other.total_bytes;
        self.malformed_count += other.malformed_count;
        // Keep only the first few samples; unbounded retention would defeat streaming.
        let remaining = 10usize.saturating_sub(self.malformed_samples.len());
        self.malformed_samples
            .extend(other.malformed_samples.into_iter().take(remaining));
    }
}

/// Processes a chunk of staged records in parallel (rayon) into per-record
/// accumulators that are folded in chunk order, keeping results deterministic.
fn process_ingest_chunk(
    chunk: Vec<IndexedRecord>,
    config: &TrainingConfig,
    repeat: usize,
) -> IngestAccumulator {
    use rayon::prelude::*;

    let parts: Vec<IngestAccumulator> = chunk
        .into_par_iter()
        .map(|item| {
            let mut acc = IngestAccumulator::default();
            match item.record {
                Ok(record) => {
                    // Bytes are counted for every well-formed record, even if sampled out.
                    acc.total_bytes = record.text.len();

                    // Deterministic probabilistic sampling for weight < 1.0,
                    // independent of processing order.
                    if item.weight < 1.0
                        && record_sample_value(config.seed, item.global_index) >= item.weight
                    {
                        return acc;
                    }

                    let reps = if item.weight >= 1.0 { repeat } else { 1 };
                    let normalized = config.normalization.normalize(&record.text);
                    let slices = config.pretokenizer.split_text(&normalized);

                    for slice in slices {
                        let bytes = slice.as_bytes();
                        if !bytes.is_empty() {
                            *acc.word_counts.entry(bytes.to_vec()).or_insert(0) += reps;
                        }
                    }
                    acc
                }
                Err(err) => {
                    acc.malformed_count = 1;
                    acc.malformed_samples.push(format!(
                        "{}:{}: {}",
                        err.file_path, err.line_number, err.reason
                    ));
                    acc
                }
            }
        })
        .collect();

    let mut out = IngestAccumulator::default();
    for part in parts {
        out.merge(part);
    }
    out
}

/// Merges a chunk accumulator into the global ingestion state (sequential, chunk order).
fn absorb_ingest_accumulator(
    acc: IngestAccumulator,
    word_counts: &mut HashMap<Vec<u8>, usize>,
    total_bytes: &mut usize,
    malformed_count: &mut usize,
    malformed_sample: &mut Vec<String>,
) {
    for (k, v) in acc.word_counts {
        *word_counts.entry(k).or_insert(0) += v;
    }
    *total_bytes += acc.total_bytes;
    *malformed_count += acc.malformed_count;
    let remaining = 10usize.saturating_sub(malformed_sample.len());
    malformed_sample.extend(acc.malformed_samples.into_iter().take(remaining));
}

/// Trains a Neuratal Tokenizer model on a corpus manifest, optionally printing
/// progress information to stderr (record counts, byte totals, unique pre-tokens,
/// merge progress vs. the known maximum, and elapsed time).
///
/// Ingestion is parallelized with Rayon over fixed-size record chunks using
/// thread-local frequency maps that are merged deterministically, so the resulting
/// model is byte-identical to sequential training with the same seed and corpus.
pub fn train_from_manifest_with_progress(
    manifest: &CorpusManifest,
    config: &TrainingConfig,
    progress: bool,
) -> Result<(Tokenizer, TrainingStats), String> {
    train_from_manifest_with_progress_and_min_word_freq(manifest, config, progress, 1)
}

/// Same as [`train_from_manifest_with_progress`], but drops unique pre-tokens
/// whose corpus frequency is below `min_word_frequency` before building the
/// pair index. This is a standard BPE memory/quality control: singleton typos,
/// hashes and one-off identifiers do not materially affect merge selection but
/// dominate unique-pre-token memory on large corpora.
pub fn train_from_manifest_with_progress_and_min_word_freq(
    manifest: &CorpusManifest,
    config: &TrainingConfig,
    progress: bool,
    min_word_frequency: usize,
) -> Result<(Tokenizer, TrainingStats), String> {
    train_from_manifest_with_progress_and_script_aware_min_word_freq(
        manifest,
        config,
        progress,
        min_word_frequency,
        None,
    )
}

/// Same as [`train_from_manifest_with_progress_and_min_word_freq`], with an
/// optional, lower frequency floor for pre-tokens composed entirely of CJK
/// (Han/Hiragana/Katakana/Hangul) characters: `cjk_min_word_frequency`. A flat
/// floor silently discards all pairs inside unsegmented CJK runs; giving them
/// their own floor preserves them at lower memory cost than `min-word-freq 1`.
pub fn train_from_manifest_with_progress_and_script_aware_min_word_freq(
    manifest: &CorpusManifest,
    config: &TrainingConfig,
    progress: bool,
    min_word_frequency: usize,
    cjk_min_word_frequency: Option<usize>,
) -> Result<(Tokenizer, TrainingStats), String> {
    let start_time = Instant::now();
    let mut last_report = Instant::now() - Duration::from_secs(10); // report immediately on first chunk

    let num_special = config.special_tokens.len();
    if config.target_vocab_size < 256 + num_special {
        return Err(format!(
            "Target vocabulary size ({}) cannot be less than base bytes (256) + special tokens ({}) = {}",
            config.target_vocab_size,
            num_special,
            256 + num_special
        ));
    }

    let max_merges = config.target_vocab_size - 256 - num_special;

    let mut word_counts: HashMap<Vec<u8>, usize> = HashMap::new();
    let mut total_records = 0usize;
    let mut total_bytes = 0usize;
    let mut malformed_count = 0usize;
    let mut malformed_sample: Vec<String> = Vec::new();
    let mut global_index: u64 = 0;

    // 1. Ingest corpus records in deterministic global order; count unique
    //    pre-token words in parallel over fixed-size chunks.
    for entry in &manifest.entries {
        let format = CorpusFormat::from_path(&entry.path);
        let mut reader = StreamingCorpusReader::open(&entry.path, format, entry.text_field.clone())
            .map_err(|e| format!("Failed to open corpus file {:?}: {}", entry.path, e))?;

        let weight = entry.weight.max(0.0);
        let weight_multiplier = weight.round() as usize;
        let repeat = if weight >= 1.0 {
            weight_multiplier.max(1)
        } else {
            1
        };

        let mut chunk: Vec<IndexedRecord> = Vec::with_capacity(INGEST_CHUNK_SIZE);

        while let Some(record_res) = reader.next_record() {
            let item_index = global_index;
            global_index += 1;
            total_records += 1;

            let item = match record_res {
                Ok(record) => IndexedRecord {
                    global_index: item_index,
                    weight,
                    record: Ok(record),
                },
                Err(err) => IndexedRecord {
                    global_index: item_index,
                    weight,
                    record: Err(err),
                },
            };
            chunk.push(item);

            if chunk.len() >= INGEST_CHUNK_SIZE {
                let acc = process_ingest_chunk(std::mem::take(&mut chunk), config, repeat);
                absorb_ingest_accumulator(
                    acc,
                    &mut word_counts,
                    &mut total_bytes,
                    &mut malformed_count,
                    &mut malformed_sample,
                );
                if progress && last_report.elapsed() >= Duration::from_millis(500) {
                    eprintln!(
                        "[train] ingesting: records={} bytes={} unique_pre_tokens={} malformed={} elapsed={:.1}s",
                        total_records,
                        total_bytes,
                        word_counts.len(),
                        malformed_count,
                        start_time.elapsed().as_secs_f64()
                    );
                    last_report = Instant::now();
                }
                chunk = Vec::with_capacity(INGEST_CHUNK_SIZE);
            }
        }

        if !chunk.is_empty() {
            let acc = process_ingest_chunk(chunk, config, repeat);
            absorb_ingest_accumulator(
                acc,
                &mut word_counts,
                &mut total_bytes,
                &mut malformed_count,
                &mut malformed_sample,
            );
        }
    }

    if progress {
        eprintln!(
            "[train] ingestion complete: records={} bytes={} unique_pre_tokens={} malformed={} elapsed={:.1}s",
            total_records,
            total_bytes,
            word_counts.len(),
            malformed_count,
            start_time.elapsed().as_secs_f64()
        );
    }

    let unique_words_count = word_counts.len();
    let pruned_words_count = if min_word_frequency > 1 || cjk_min_word_frequency.is_some() {
        let before = word_counts.len();
        let mut cjk_kept = 0usize;
        word_counts.retain(|token, count| {
            let floor = match cjk_min_word_frequency {
                Some(f) if pre_token_is_all_cjk(token) => {
                    if *count < min_word_frequency {
                        cjk_kept += 1;
                    }
                    f
                }
                _ => min_word_frequency,
            };
            *count >= floor
        });
        let pruned = before - word_counts.len();
        if progress && cjk_kept > 0 {
            eprintln!(
                "[train] CJK script-aware floor kept {} pre-tokens below the flat frequency {}",
                cjk_kept, min_word_frequency
            );
        }
        pruned
    } else {
        0
    };
    let training_words_count = word_counts.len();
    if progress && pruned_words_count > 0 {
        eprintln!(
            "[train] pruned {} unique pre-tokens with frequency < {}; {} remain",
            pruned_words_count, min_word_frequency, training_words_count
        );
    }

    // 2. Build BPE pair trainer
    let mut pair_trainer = BpePairTrainer::from_word_counts(word_counts);
    let initial_pair_count = pair_trainer.pair_frequencies_count();
    let initial_occurrences_count = pair_trainer.total_occurrences_count();

    // 3. Initialize Tokenizer components
    let mut vocab = Vocabulary::new_with_base_bytes();
    let mut merges = MergeTable::new();

    let mut merges_learned = 0usize;

    // 4. Execute iterative BPE merges
    let mut skipped_duplicate_merges = 0usize;
    let mut last_merge_report = Instant::now() - Duration::from_secs(10); // report immediately on first merge
    while merges_learned < max_merges {
        let next_token_id = (256 + merges_learned) as u32;

        // Peek at the candidate and verify it would not re-create an existing
        // token's byte sequence before any trainer state is rewritten. Different
        // segmentations of the same byte string can both become merge candidates;
        // such pairs are redundant and are dropped instead of aborting training.
        let (best_pair, _freq) = match pair_trainer.peek_merge(config.min_frequency) {
            Some(candidate) => candidate,
            None => break,
        };

        let left_bytes = vocab
            .get_bytes(best_pair.0)
            .ok_or_else(|| format!("Token ID {} not found in vocab", best_pair.0))?
            .to_vec();
        let right_bytes = vocab
            .get_bytes(best_pair.1)
            .ok_or_else(|| format!("Token ID {} not found in vocab", best_pair.1))?
            .to_vec();

        let mut combined_bytes = left_bytes;
        combined_bytes.extend_from_slice(&right_bytes);

        if let Some(existing_id) = vocab.get_id(&combined_bytes) {
            skipped_duplicate_merges += 1;
            if progress {
                eprintln!(
                    "[train] skipping merge candidate ({}, {}) -> bytes already exist as token {} ({} skipped)",
                    best_pair.0, best_pair.1, existing_id, skipped_duplicate_merges
                );
            }
            pair_trainer.discard_pair(best_pair);
            continue;
        }

        pair_trainer.apply_merge(next_token_id);
        merges.add_merge(best_pair.0, best_pair.1, next_token_id)?;
        vocab.add_token(next_token_id, combined_bytes)?;
        merges_learned += 1;

        if progress && last_merge_report.elapsed() >= Duration::from_millis(500) {
            eprintln!(
                "[train] learning merges: {}/{} merges learned (id {}), pair_frequencies={} queued_entries={} skipped_duplicates={} elapsed={:.1}s",
                merges_learned,
                max_merges,
                next_token_id,
                pair_trainer.pair_frequencies_count(),
                pair_trainer.priority_queue_size(),
                skipped_duplicate_merges,
                start_time.elapsed().as_secs_f64()
            );
            last_merge_report = Instant::now();
        }
    }

    // 5. Register special tokens with contiguous IDs following the learned merges.
    // If a special token's byte sequence already exists in the vocabulary (e.g. the
    // corpus literally contained a special token string and BPE merged it), the
    // special token reuses that existing ID instead of adding a duplicate token
    // (or breaking ID contiguity); decoding special tokens does not depend on a
    // dedicated vocabulary entry.
    let mut special_registry = SpecialTokenRegistry::new();
    let mut next_special_id = vocab.len() as u32;

    for token_str in config.special_tokens.iter() {
        let special_id = match vocab.get_id(token_str.as_bytes()) {
            Some(existing) => {
                if existing < 256 {
                    return Err(format!(
                        "Special token '{}' duplicates base-byte token ID {}; choose different special token content",
                        token_str, existing
                    ));
                }
                if progress {
                    eprintln!(
                        "[train] note: special token '{}' reuses existing vocabulary token ID {}",
                        token_str, existing
                    );
                }
                existing
            }
            None => {
                let id = next_special_id;
                vocab.add_token(id, token_str.as_bytes().to_vec())?;
                next_special_id = id + 1;
                id
            }
        };
        special_registry.register(token_str, special_id)?;
    }

    let final_vocab_size = vocab.len();
    let elapsed = start_time.elapsed().as_millis();

    if progress {
        eprintln!(
            "[train] complete: merges={} skipped_duplicates={} final_vocab_size={} elapsed={:.3}s",
            merges_learned,
            skipped_duplicate_merges,
            final_vocab_size,
            start_time.elapsed().as_secs_f64()
        );
    }

    let tokenizer = Tokenizer::new(
        vocab,
        merges,
        special_registry,
        config.pretokenizer,
        config.normalization,
    )?;

    let stats = TrainingStats {
        total_records_read: total_records,
        total_bytes_processed: total_bytes,
        malformed_records_count: malformed_count,
        malformed_records_sample: malformed_sample,
        unique_words_count,
        pruned_words_count,
        training_words_count,
        initial_pair_count,
        initial_occurrences_count,
        merges_learned,
        final_vocab_size,
        elapsed_millis: elapsed,
        skipped_duplicate_merges,
    };

    Ok((tokenizer, stats))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::training::sampling::CorpusManifestEntry;
    use std::fs::File;
    use std::io::Write;

    #[test]
    fn test_train_from_manifest_basic() {
        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("mini_corpus.txt");
        {
            let mut f = File::create(&file_path).unwrap();
            for _ in 0..10 {
                writeln!(f, "the quick brown fox jumps over the lazy dog").unwrap();
            }
        }

        let mut manifest = CorpusManifest::new(42);
        manifest.add_entry(CorpusManifestEntry {
            path: file_path.clone(),
            weight: 1.0,
            language: Some("en".to_string()),
            domain: Some("general".to_string()),
            text_field: None,
        });

        let config = TrainingConfig {
            target_vocab_size: 270, // 256 base + merges + 2 special
            min_frequency: 2,
            pretokenizer: PretokenizationMode::RawByte,
            normalization: Normalization::None,
            special_tokens: vec!["<|bos|>".to_string(), "<|eos|>".to_string()],
            seed: 42,
        };

        let (tokenizer, stats) = train_from_manifest(&manifest, &config).unwrap();
        assert_eq!(stats.final_vocab_size, 270);
        assert_eq!(stats.merges_learned, 12);
        assert_eq!(tokenizer.vocab_size(), 270);

        // Exact round-trip test on trained model
        let sample = "the quick brown fox";
        let encoded = tokenizer.encode_text(sample).unwrap();
        let decoded = tokenizer.decode_text(&encoded).unwrap();
        assert_eq!(decoded, sample);

        let _ = std::fs::remove_file(file_path);
    }

    /// Sequential reference implementation used to verify that the parallel
    /// chunked ingestion path produces byte-identical counts.
    fn sequential_reference_counts(
        records: &[(u64, f64, &str)],
        config: &TrainingConfig,
    ) -> HashMap<Vec<u8>, usize> {
        let mut counts: HashMap<Vec<u8>, usize> = HashMap::new();
        for &(index, weight, text) in records {
            if weight < 1.0 && record_sample_value(config.seed, index) >= weight {
                continue;
            }
            let repeat = if weight >= 1.0 {
                (weight.round() as usize).max(1)
            } else {
                1
            };
            let normalized = config.normalization.normalize(text);
            for slice in config.pretokenizer.split_text(&normalized) {
                let bytes = slice.as_bytes();
                if !bytes.is_empty() {
                    *counts.entry(bytes.to_vec()).or_insert(0) += repeat;
                }
            }
        }
        counts
    }

    #[test]
    fn test_parallel_ingest_matches_sequential_reference() {
        let texts = [
            "the quick brown fox jumps over the lazy dog",
            "Türkçe metin: Şemsi Paşa Pasajı 12345",
            "Алгоритмы машинного обучения",
            "def solve(x):\n    return x * 42",
            "Emoji 👨‍👩‍👧‍👦 and numbers 3.14159e+10",
            "the quick brown fox",
            "another line with some words for counting",
        ];

        let config = TrainingConfig {
            target_vocab_size: 300,
            min_frequency: 1,
            pretokenizer: PretokenizationMode::ConservativeUnicode,
            normalization: Normalization::None,
            special_tokens: vec![],
            seed: 7,
        };

        for (label, weight) in [("full", 1.0), ("sampled", 0.5)] {
            let records: Vec<(u64, f64, &str)> = texts
                .iter()
                .enumerate()
                .map(|(i, t)| (i as u64, weight, *t))
                .collect();

            let indexed: Vec<IndexedRecord> = records
                .iter()
                .map(|&(index, w, text)| IndexedRecord {
                    global_index: index,
                    weight: w,
                    record: Ok(crate::training::corpus::CorpusRecord {
                        text: text.to_string(),
                        language: None,
                        domain: None,
                        file_path: "test".to_string(),
                        line_number: index as usize + 1,
                    }),
                })
                .collect();

            let repeat = if weight >= 1.0 {
                (weight.round() as usize).max(1)
            } else {
                1
            };
            let parallel = process_ingest_chunk(indexed, &config, repeat);
            let sequential = sequential_reference_counts(&records, &config);

            assert_eq!(
                parallel.word_counts, sequential,
                "parallel and sequential ingestion disagree for weight={} ({})",
                weight, label
            );
        }
    }

    #[test]
    fn test_record_sample_value_is_deterministic_and_in_range() {
        for seed in [0, 1, 42, 999, u64::MAX] {
            for index in [0, 1, 2, 1000, u64::MAX] {
                let v = record_sample_value(seed, index);
                assert!((0.0..1.0).contains(&v), "sample value {} out of range", v);
                assert_eq!(v, record_sample_value(seed, index), "not deterministic");
            }
        }
    }
}
