//! Decoding module for Neuratal Tokenizer.

pub mod decoder;

pub use decoder::{DecodeOptions, decode_bytes, decode_text, decode_text_lossy};
