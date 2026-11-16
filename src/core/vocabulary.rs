//! Vocabulary storage and bidirectional token <-> byte slice mapping.
//!
//! In Neuratal Tokenizer, token IDs 0..=255 are guaranteed to map to individual bytes `0x00..=0xFF`.
//! Higher token IDs represent either learned BPE byte sequences or special tokens.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub type TokenId = u32;

/// Bidirectional vocabulary mapping between TokenId and byte sequences.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Vocabulary {
    /// Mapping from TokenId to byte sequence. Index equals TokenId for dense IDs.
    id_to_bytes: Vec<Vec<u8>>,
    /// Reverse mapping from byte sequence to TokenId.
    bytes_to_id: HashMap<Vec<u8>, TokenId>,
}

#[derive(Serialize, Deserialize)]
struct VocabularyDef {
    id_to_bytes: Vec<Vec<u8>>,
}

impl Serialize for Vocabulary {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        VocabularyDef {
            id_to_bytes: self.id_to_bytes.clone(),
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Vocabulary {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let def = VocabularyDef::deserialize(deserializer)?;
        let mut bytes_to_id = HashMap::with_capacity(def.id_to_bytes.len());
        for (id, bytes) in def.id_to_bytes.iter().enumerate() {
            bytes_to_id.insert(bytes.clone(), id as TokenId);
        }
        Ok(Self {
            id_to_bytes: def.id_to_bytes,
            bytes_to_id,
        })
    }
}

impl Default for Vocabulary {
    fn default() -> Self {
        Self::new_with_base_bytes()
    }
}

impl Vocabulary {
    /// Creates a new vocabulary pre-seeded with all 256 base byte tokens (IDs 0..=255).
    pub fn new_with_base_bytes() -> Self {
        let mut id_to_bytes = Vec::with_capacity(256);
        let mut bytes_to_id = HashMap::with_capacity(256);

        for b in 0u8..=255u8 {
            let byte_slice = vec![b];
            bytes_to_id.insert(byte_slice.clone(), b as u32);
            id_to_bytes.push(byte_slice);
        }

        Self {
            id_to_bytes,
            bytes_to_id,
        }
    }

    /// Adds a new token byte sequence with an explicit TokenId.
    pub fn add_token(&mut self, id: TokenId, bytes: Vec<u8>) -> Result<(), String> {
        if id < 256 {
            return Err(format!(
                "Cannot overwrite base byte token ID {}: IDs 0..=255 are strictly reserved.",
                id
            ));
        }
        if id as usize != self.id_to_bytes.len() {
            return Err(format!(
                "Non-continuous token ID allocation: expected ID {}, got {}",
                self.id_to_bytes.len(),
                id
            ));
        }
        if let Some(&existing_id) = self.bytes_to_id.get(&bytes) {
            return Err(format!(
                "Byte sequence already exists with Token ID {}",
                existing_id
            ));
        }

        self.bytes_to_id.insert(bytes.clone(), id);
        self.id_to_bytes.push(bytes);
        Ok(())
    }

    /// Retrieves the byte slice corresponding to a TokenId.
    #[inline(always)]
    pub fn get_bytes(&self, id: TokenId) -> Option<&[u8]> {
        self.id_to_bytes.get(id as usize).map(|v| v.as_slice())
    }

    /// Retrieves the TokenId for an exact byte sequence.
    #[inline(always)]
    pub fn get_id(&self, bytes: &[u8]) -> Option<TokenId> {
        self.bytes_to_id.get(bytes).copied()
    }

    /// Returns the total number of tokens in the vocabulary.
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.id_to_bytes.len()
    }

    /// Checks if the vocabulary is empty.
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.id_to_bytes.is_empty()
    }

    /// Validates the structural integrity of the vocabulary.
    pub fn validate(&self) -> Result<(), String> {
        if self.id_to_bytes.len() < 256 {
            return Err(format!(
                "Vocabulary has fewer than 256 tokens: found {}",
                self.id_to_bytes.len()
            ));
        }

        // Validate base bytes 0..=255
        for b in 0u8..=255u8 {
            let id = b as usize;
            let slice = &self.id_to_bytes[id];
            if slice.len() != 1 || slice[0] != b {
                return Err(format!(
                    "Base byte corruption: ID {} maps to {:?}, expected [{}]",
                    id, slice, b
                ));
            }
        }

        if self.bytes_to_id.len() != self.id_to_bytes.len() {
            return Err(format!(
                "Vocabulary mapping discrepancy: id_to_bytes count ({}) != bytes_to_id count ({})",
                self.id_to_bytes.len(),
                self.bytes_to_id.len()
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vocabulary_base_bytes_initialization() {
        let vocab = Vocabulary::new_with_base_bytes();
        assert_eq!(vocab.len(), 256);
        assert!(vocab.validate().is_ok());

        for b in 0u8..=255u8 {
            assert_eq!(vocab.get_bytes(b as u32), Some(&[b][..]));
            assert_eq!(vocab.get_id(&[b]), Some(b as u32));
        }
    }

    #[test]
    fn test_vocabulary_continuous_allocation() {
        let mut vocab = Vocabulary::new_with_base_bytes();
        assert!(vocab.add_token(256, b"th".to_vec()).is_ok());
        assert!(vocab.add_token(257, b"the".to_vec()).is_ok());
        // Gap detection
        assert!(vocab.add_token(260, b"gap".to_vec()).is_err());
        // Base byte overwrite attempt
        assert!(vocab.add_token(10, b"bad".to_vec()).is_err());
    }

    #[test]
    fn test_vocabulary_serde_roundtrip() {
        let mut vocab = Vocabulary::new_with_base_bytes();
        vocab.add_token(256, b"th".to_vec()).unwrap();
        vocab.add_token(257, b"the".to_vec()).unwrap();

        let json = serde_json::to_string(&vocab).unwrap();
        let loaded: Vocabulary = serde_json::from_str(&json).unwrap();

        assert_eq!(loaded.len(), vocab.len());
        assert_eq!(loaded.get_bytes(256), Some(&b"th"[..]));
        assert_eq!(loaded.get_id(b"the"), Some(257));
    }
}
