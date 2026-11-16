//! # Neuratal Tokenizer
//!
//! An independent, production-grade, lossless byte-level BPE tokenizer implemented
//! from scratch in pure, safe Rust.
//!
//! Designed for modern autoregressive language models with strict support for
//! multilingual text, source code, mathematical notation, arbitrary Unicode sequences,
//! and raw binary data.

pub mod benchmarking;
pub mod core;
pub mod decoding;
pub mod encoding;
pub mod ffi;
pub mod serialization;
pub mod sha256;
pub mod training;
pub mod validation;

#[cfg(feature = "pyo3")]
mod python_module;

pub use benchmarking::metrics::{
    CategoryMetrics, ComparativeBenchmarkReport, EvalCategory, EvalSample, StrategyBenchmarkResult,
    get_standard_evaluation_suite, run_pretokenization_comparison,
};

pub use core::bytes::{
    BASE_BYTE_COUNT, byte_to_token_id, format_bytes_for_display, token_id_to_byte,
};
pub use core::merges::{MergeRule, MergeTable};
pub use core::special_tokens::{
    AllowedSpecial, SPECIAL_ASSISTANT, SPECIAL_BOS, SPECIAL_END_THINK, SPECIAL_EOS, SPECIAL_PAD,
    SPECIAL_SYSTEM, SPECIAL_THINK, SPECIAL_TOOL, SPECIAL_UNK, SPECIAL_USER, SpecialToken,
    SpecialTokenRegistry,
};
pub use core::vocabulary::{TokenId, Vocabulary};
pub use decoding::decoder::{DecodeOptions, decode_bytes, decode_text, decode_text_lossy};
pub use encoding::encoder::{EncodeOptions, TokenizerError, encode_bytes, encode_text};
pub use encoding::normalizer::Normalization;
pub use encoding::pretokenizer::PretokenizationMode;
pub use serialization::format::{
    FORMAT_VERSION, FORMAT_VERSION_V1, FORMAT_VERSION_V2, FORMAT_VERSION_V3, HEADER_SIZE,
    MAGIC_BYTES, MAGIC_BYTES_V1, MAX_PAYLOAD_BYTES, ModelMetadata, NTOK_EXTENSION, RZT_EXTENSION,
    compute_crc32, load_ntok_file, save_ntok_file,
};
pub use sha256::{sha256, sha256_hex};
pub use training::bpe_trainer::{
    TrainingConfig, TrainingStats, train_from_manifest, train_from_manifest_with_progress,
    train_from_manifest_with_progress_and_min_word_freq,
    train_from_manifest_with_progress_and_script_aware_min_word_freq,
};
pub use training::corpus::{CorpusFormat, CorpusRecord, CorpusRecordError, StreamingCorpusReader};
pub use training::pair_counter::{BpePairTrainer, Occurrence, SymbolNode, Word};
pub use training::sampling::{CorpusManifest, CorpusManifestEntry, DeterministicRng};
pub use validation::{
    ValidationCheck, ValidationReport, validate_model_structure, validate_tokenizer,
};

use serde::{Deserialize, Serialize};
use std::path::Path;

/// The unified Neuratal Tokenizer engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tokenizer {
    vocab: Vocabulary,
    merges: MergeTable,
    special_tokens: SpecialTokenRegistry,
    pretokenizer: PretokenizationMode,
    normalization: Normalization,
    /// Embedded model metadata (format v3 payloads carry it; v1/v2 leave it `None`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    metadata: Option<ModelMetadata>,
}

impl Default for Tokenizer {
    fn default() -> Self {
        Self::new_base()
    }
}

impl Tokenizer {
    /// Creates a base tokenizer with 256 byte tokens and no learned merges.
    pub fn new_base() -> Self {
        Self {
            vocab: Vocabulary::new_with_base_bytes(),
            merges: MergeTable::new(),
            special_tokens: SpecialTokenRegistry::new(),
            pretokenizer: PretokenizationMode::default(),
            normalization: Normalization::None,
            metadata: None,
        }
    }

    /// Constructs a tokenizer with explicit components.
    pub fn new(
        vocab: Vocabulary,
        merges: MergeTable,
        special_tokens: SpecialTokenRegistry,
        pretokenizer: PretokenizationMode,
        normalization: Normalization,
    ) -> Result<Self, String> {
        vocab.validate()?;
        Ok(Self {
            vocab,
            merges,
            special_tokens,
            pretokenizer,
            normalization,
            metadata: None,
        })
    }

    /// Returns the embedded model metadata, if any.
    pub fn metadata(&self) -> Option<&ModelMetadata> {
        self.metadata.as_ref()
    }

    /// Attaches (or replaces) the embedded model metadata. v3 payloads save it;
    /// the metadata is validated on load by `validate_model_structure`.
    pub fn set_metadata(&mut self, metadata: ModelMetadata) {
        self.metadata = Some(metadata);
    }

    /// Encodes a UTF-8 text string into token IDs using default encoding options.
    pub fn encode_text(&self, text: &str) -> Result<Vec<TokenId>, TokenizerError> {
        let options = EncodeOptions {
            allowed_special: AllowedSpecial::All,
            normalization: self.normalization,
            pretokenizer: self.pretokenizer,
        };
        encode_text(text, &self.merges, &self.special_tokens, &options)
    }

    /// Encodes a UTF-8 text string with custom encoding options.
    pub fn encode_text_with_options(
        &self,
        text: &str,
        options: &EncodeOptions,
    ) -> Result<Vec<TokenId>, TokenizerError> {
        encode_text(text, &self.merges, &self.special_tokens, options)
    }

    /// Parallel batch encoding of multiple texts across threads using Rayon.
    pub fn encode_batch(&self, texts: &[&str]) -> Vec<Result<Vec<TokenId>, TokenizerError>> {
        use rayon::prelude::*;
        texts.par_iter().map(|t| self.encode_text(t)).collect()
    }

    /// Parallel batch encoding with explicit encoding options.
    pub fn encode_batch_with_options(
        &self,
        texts: &[&str],
        options: &EncodeOptions,
    ) -> Vec<Result<Vec<TokenId>, TokenizerError>> {
        use rayon::prelude::*;
        texts
            .par_iter()
            .map(|t| self.encode_text_with_options(t, options))
            .collect()
    }

    /// Decodes a sequence of token IDs into a verified UTF-8 string.
    pub fn decode_text(&self, ids: &[TokenId]) -> Result<String, TokenizerError> {
        let options = DecodeOptions::default();
        decode_text(ids, &self.vocab, &self.special_tokens, &options)
    }

    /// Decodes a sequence of token IDs with custom decoding options.
    pub fn decode_text_with_options(
        &self,
        ids: &[TokenId],
        options: &DecodeOptions,
    ) -> Result<String, TokenizerError> {
        decode_text(ids, &self.vocab, &self.special_tokens, options)
    }

    /// Parallel batch decoding of multiple token sequences into strings.
    pub fn decode_batch(&self, batches: &[&[TokenId]]) -> Vec<Result<String, TokenizerError>> {
        use rayon::prelude::*;
        batches.par_iter().map(|b| self.decode_text(b)).collect()
    }

    /// Encodes arbitrary raw bytes into token IDs (supports invalid UTF-8).
    pub fn encode_bytes(&self, bytes: &[u8]) -> Vec<TokenId> {
        encode_bytes(bytes, &self.merges, &self.pretokenizer)
    }

    /// Decodes token IDs back into raw bytes (supports invalid UTF-8).
    pub fn decode_bytes(&self, ids: &[TokenId]) -> Result<Vec<u8>, TokenizerError> {
        let options = DecodeOptions::default();
        decode_bytes(ids, &self.vocab, &self.special_tokens, &options)
    }

    /// Decodes token IDs back into raw bytes with custom options.
    pub fn decode_bytes_with_options(
        &self,
        ids: &[TokenId],
        options: &DecodeOptions,
    ) -> Result<Vec<u8>, TokenizerError> {
        decode_bytes(ids, &self.vocab, &self.special_tokens, options)
    }

    /// Registers a new special token.
    pub fn register_special_token(&mut self, content: &str, id: TokenId) -> Result<(), String> {
        self.special_tokens.register(content, id)?;
        Ok(())
    }

    /// Adds a learned merge and expands the vocabulary.
    pub fn add_merge(&mut self, left: TokenId, right: TokenId) -> Result<TokenId, String> {
        let left_bytes = self
            .vocab
            .get_bytes(left)
            .ok_or_else(|| format!("Left token ID {} not found in vocabulary", left))?
            .to_vec();
        let right_bytes = self
            .vocab
            .get_bytes(right)
            .ok_or_else(|| format!("Right token ID {} not found in vocabulary", right))?
            .to_vec();

        let mut combined_bytes = left_bytes;
        combined_bytes.extend_from_slice(&right_bytes);

        let new_id = self.vocab.len() as TokenId;
        self.merges.add_merge(left, right, new_id)?;
        self.vocab.add_token(new_id, combined_bytes)?;
        Ok(new_id)
    }

    /// Returns a reference to the vocabulary.
    pub fn vocab(&self) -> &Vocabulary {
        &self.vocab
    }

    /// Returns a reference to the merge table.
    pub fn merges(&self) -> &MergeTable {
        &self.merges
    }

    /// Returns a reference to the special token registry.
    pub fn special_tokens(&self) -> &SpecialTokenRegistry {
        &self.special_tokens
    }

    /// Returns the total vocabulary size.
    pub fn vocab_size(&self) -> usize {
        self.vocab.len()
    }

    /// Returns the active pre-tokenization mode.
    pub fn pretokenizer(&self) -> PretokenizationMode {
        self.pretokenizer
    }

    /// Sets the pre-tokenization mode.
    pub fn set_pretokenizer(&mut self, mode: PretokenizationMode) {
        self.pretokenizer = mode;
    }

    /// Returns the active normalization mode.
    pub fn normalization(&self) -> Normalization {
        self.normalization
    }

    /// Sets the normalization mode.
    pub fn set_normalization(&mut self, norm: Normalization) {
        self.normalization = norm;
    }

    /// Exports a smaller-vocabulary model from a trained model's learned merge order.
    ///
    /// The exported model keeps the **first `budget` merges in rank order** (their
    /// token IDs are preserved: merges at ranks 0..budget have targets 256..256+budget-1),
    /// the 256 immutable base byte tokens, and all special tokens (renumbered compactly
    /// after the kept merges if their IDs moved; special tokens whose content bytes
    /// coincide with a kept merge token reuse that token's ID instead, exactly as the
    /// trainer does).
    ///
    /// `target_vocab_size` is a *budget*: if the model does not contain enough merges,
    /// the exported model simply keeps them all. The special-token count is preserved.
    /// Normalization and pretokenizer are preserved. The result is a fully valid,
    /// independent model that can be saved, loaded, and used on its own.
    pub fn export_trimmed(&self, target_vocab_size: usize) -> Result<Tokenizer, String> {
        let num_special = self.special_tokens.len();
        let min_size = 256 + num_special;
        if target_vocab_size < min_size {
            return Err(format!(
                "Cannot export to vocabulary size {}: minimum is 256 base bytes + {} special tokens = {}",
                target_vocab_size, num_special, min_size
            ));
        }

        let merge_budget = (target_vocab_size - 256 - num_special).min(self.merges.len());

        // 1. Base bytes + the first `merge_budget` kept merge targets (IDs are preserved).
        let mut vocab = Vocabulary::new_with_base_bytes();
        for id in 256..(256 + merge_budget) {
            let bytes = self
                .vocab
                .get_bytes(id as u32)
                .ok_or_else(|| format!("Merge target ID {} missing from vocabulary", id))?
                .to_vec();
            vocab.add_token(id as u32, bytes)?;
        }

        // 2. Kept merges, preserving rank order exactly.
        let mut merges = MergeTable::new();
        for rule in self.merges.rules().iter().take(merge_budget) {
            merges.add_merge(rule.left, rule.right, rule.target)?;
        }

        // 3. Special tokens: renumber compactly, reusing existing IDs where the
        //    special content bytes coincide with a kept token (same rule as training).
        let mut special_registry = SpecialTokenRegistry::new();
        let mut next_id = (256 + merge_budget) as u32;
        for st in self.special_tokens.all_tokens() {
            let id = match vocab.get_id(st.content.as_bytes()) {
                Some(existing) => existing,
                None => {
                    let id = next_id;
                    vocab.add_token(id, st.content.as_bytes().to_vec())?;
                    next_id += 1;
                    id
                }
            };
            special_registry.register(&st.content, id)?;
        }

        // Vocabulary is validated transitively by Tokenizer::new.
        let mut out = Tokenizer::new(
            vocab,
            merges,
            special_registry,
            self.pretokenizer,
            self.normalization,
        )?;
        // Exported smaller models are independent artifacts: they carry their
        // own derived metadata (correct vocab size / merge count), not the
        // parent's, so a reader can tell exactly how they were produced.
        if let Some(parent) = &self.metadata {
            let actual_vocab = out.vocab_size() as u32;
            let actual_merges = out.merges().len() as u32;
            let actual_special = out.special_tokens.len() as u32;
            let base_name = if parent.model_name.is_empty() {
                "ntokenizer".to_string()
            } else {
                parent.model_name.clone()
            };
            out.set_metadata(parent.for_export(
                &format!("{}-{}", base_name, actual_vocab),
                actual_vocab,
                actual_merges,
                actual_special,
            ));
        }
        Ok(out)
    }

    /// Saves the tokenizer model to an `.ntok` binary file.
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), String> {
        save_ntok_file(self, path)
    }

    /// Loads a tokenizer model from an `.ntok` binary file.
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        load_ntok_file(path)
    }

    /// Serializes the tokenizer to a JSON string.
    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string_pretty(self).map_err(|e| e.to_string())
    }

    /// Deserializes a tokenizer from a JSON string.
    pub fn from_json(json_str: &str) -> Result<Self, String> {
        let tok: Self = serde_json::from_str(json_str).map_err(|e| e.to_string())?;
        tok.vocab.validate()?;
        Ok(tok)
    }
}
