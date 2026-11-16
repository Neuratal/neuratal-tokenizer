//! Encoding module for Neuratal Tokenizer.

pub mod encoder;
pub mod normalizer;
pub mod pretokenizer;

pub use encoder::{EncodeOptions, TokenizerError, encode_bytes, encode_text};
pub use normalizer::Normalization;
pub use pretokenizer::PretokenizationMode;
