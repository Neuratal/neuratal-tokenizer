//! Serialization and deserialization module for Neuratal Tokenizer.

pub mod format;

pub use format::{
    FORMAT_VERSION, FORMAT_VERSION_V1, HEADER_SIZE, MAGIC_BYTES, MAGIC_BYTES_V1, MAX_PAYLOAD_BYTES,
    ModelMetadata, NTOK_EXTENSION, RZT_EXTENSION, compute_crc32, load_ntok_file, save_ntok_file,
};
