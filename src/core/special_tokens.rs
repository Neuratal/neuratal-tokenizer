//! Special token registry and management.
//!
//! Enforces strict separation between base byte tokens (0..=255), learned BPE tokens,
//! and special tokens. Special tokens cannot collide with base bytes or each other.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Standard well-known special token literals.
pub const SPECIAL_BOS: &str = "<|bos|>";
pub const SPECIAL_EOS: &str = "<|eos|>";
pub const SPECIAL_PAD: &str = "<|pad|>";
pub const SPECIAL_UNK: &str = "<|unk|>";
pub const SPECIAL_SYSTEM: &str = "<|system|>";
pub const SPECIAL_USER: &str = "<|user|>";
pub const SPECIAL_ASSISTANT: &str = "<|assistant|>";
pub const SPECIAL_TOOL: &str = "<|tool|>";
pub const SPECIAL_THINK: &str = "<|think|>";
pub const SPECIAL_END_THINK: &str = "<|/think|>";

/// Configuration specifying which special tokens are allowed during encoding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AllowedSpecial {
    /// All registered special tokens are parsed into special token IDs.
    All,
    /// No special tokens are parsed; literal strings like "<|bos|>" are encoded as normal text.
    None,
    /// Only the specified special token strings are recognized; others are treated as normal text.
    Explicit(HashSet<String>),
}

/// A registered special token definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpecialToken {
    pub id: u32,
    pub content: String,
    pub lstrip: bool,
    pub rstrip: bool,
}

/// Registry managing all special tokens for a tokenizer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpecialTokenRegistry {
    /// Mapping from special token content string to its TokenId.
    token_to_id: HashMap<String, u32>,
    /// Mapping from TokenId to special token content string.
    id_to_token: HashMap<u32, String>,
    /// Ordered list of special tokens.
    tokens: Vec<SpecialToken>,
}

#[derive(Serialize, Deserialize)]
struct SpecialTokenRegistryDef {
    tokens: Vec<SpecialToken>,
}

impl Serialize for SpecialTokenRegistry {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        SpecialTokenRegistryDef {
            tokens: self.tokens.clone(),
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for SpecialTokenRegistry {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let def = SpecialTokenRegistryDef::deserialize(deserializer)?;
        let mut token_to_id = HashMap::with_capacity(def.tokens.len());
        let mut id_to_token = HashMap::with_capacity(def.tokens.len());
        for st in &def.tokens {
            token_to_id.insert(st.content.clone(), st.id);
            id_to_token.insert(st.id, st.content.clone());
        }
        Ok(Self {
            token_to_id,
            id_to_token,
            tokens: def.tokens,
        })
    }
}

impl Default for SpecialTokenRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl SpecialTokenRegistry {
    /// Creates an empty special token registry.
    pub fn new() -> Self {
        Self {
            token_to_id: HashMap::new(),
            id_to_token: HashMap::new(),
            tokens: Vec::new(),
        }
    }

    /// Registers a special token with an explicit ID.
    pub fn register(&mut self, content: &str, id: u32) -> Result<(), String> {
        if id < 256 {
            return Err(format!(
                "Cannot register special token '{}' with ID {}: IDs 0..=255 are strictly reserved for base bytes.",
                content, id
            ));
        }
        if let Some(existing_id) = self.token_to_id.get(content) {
            if *existing_id != id {
                return Err(format!(
                    "Special token '{}' is already registered with ID {}",
                    content, existing_id
                ));
            }
            return Ok(());
        }
        if let Some(existing_content) = self.id_to_token.get(&id) {
            return Err(format!(
                "Token ID {} is already assigned to special token '{}'",
                id, existing_content
            ));
        }

        self.token_to_id.insert(content.to_string(), id);
        self.id_to_token.insert(id, content.to_string());
        self.tokens.push(SpecialToken {
            id,
            content: content.to_string(),
            lstrip: false,
            rstrip: false,
        });
        Ok(())
    }

    /// Returns the TokenId for a given special token string, if registered.
    pub fn get_id(&self, content: &str) -> Option<u32> {
        self.token_to_id.get(content).copied()
    }

    /// Returns the content string for a given TokenId, if it is a special token.
    pub fn get_content(&self, id: u32) -> Option<&str> {
        self.id_to_token.get(&id).map(|s| s.as_str())
    }

    /// Checks if a TokenId is a registered special token.
    pub fn is_special(&self, id: u32) -> bool {
        self.id_to_token.contains_key(&id)
    }

    /// Returns the number of registered special tokens.
    pub fn len(&self) -> usize {
        self.tokens.len()
    }

    /// Returns whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.tokens.is_empty()
    }

    /// Returns all registered special tokens.
    pub fn all_tokens(&self) -> &[SpecialToken] {
        &self.tokens
    }

    /// Finds matches of registered special tokens in the text, according to `AllowedSpecial`.
    pub fn split_text(
        &self,
        text: &str,
        allowed: &AllowedSpecial,
    ) -> Vec<(usize, usize, Option<u32>)> {
        if self.is_empty() || text.is_empty() || *allowed == AllowedSpecial::None {
            return vec![(0, text.len(), None)];
        }

        let active_tokens: Vec<(&str, u32)> = self
            .tokens
            .iter()
            .filter(|st| match allowed {
                AllowedSpecial::All => true,
                AllowedSpecial::None => false,
                AllowedSpecial::Explicit(set) => set.contains(&st.content),
            })
            .map(|st| (st.content.as_str(), st.id))
            .collect();

        if active_tokens.is_empty() {
            return vec![(0, text.len(), None)];
        }

        let mut matches = Vec::new();
        let mut search_idx = 0;

        while search_idx < text.len() {
            let mut earliest_match: Option<(usize, usize, u32)> = None;

            for &(content, id) in &active_tokens {
                if let Some(pos) = text[search_idx..].find(content) {
                    let abs_start = search_idx + pos;
                    let abs_end = abs_start + content.len();
                    match earliest_match {
                        None => earliest_match = Some((abs_start, abs_end, id)),
                        Some((best_start, best_end, _)) => {
                            if abs_start < best_start
                                || (abs_start == best_start
                                    && (abs_end - abs_start) > (best_end - best_start))
                            {
                                earliest_match = Some((abs_start, abs_end, id));
                            }
                        }
                    }
                }
            }

            match earliest_match {
                Some((start, end, id)) => {
                    if start > search_idx {
                        matches.push((search_idx, start, None));
                    }
                    matches.push((start, end, Some(id)));
                    search_idx = end;
                }
                None => {
                    matches.push((search_idx, text.len(), None));
                    break;
                }
            }
        }

        matches
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_special_tokens_cannot_use_byte_range() {
        let mut reg = SpecialTokenRegistry::new();
        for id in 0..256 {
            assert!(reg.register("<|test|>", id).is_err());
        }
        assert!(reg.register("<|bos|>", 256).is_ok());
    }

    #[test]
    fn test_special_tokens_collision_rejection() {
        let mut reg = SpecialTokenRegistry::new();
        assert!(reg.register("<|bos|>", 256).is_ok());
        assert!(reg.register("<|eos|>", 256).is_err());
        assert!(reg.register("<|bos|>", 257).is_err());
        assert!(reg.register("<|eos|>", 257).is_ok());
    }

    #[test]
    fn test_special_tokens_serde_roundtrip() {
        let mut reg = SpecialTokenRegistry::new();
        reg.register("<|bos|>", 256).unwrap();
        reg.register("<|eos|>", 257).unwrap();

        let json = serde_json::to_string(&reg).unwrap();
        let loaded: SpecialTokenRegistry = serde_json::from_str(&json).unwrap();

        assert_eq!(loaded.len(), reg.len());
        assert_eq!(loaded.get_id("<|bos|>"), Some(256));
        assert_eq!(loaded.get_content(257), Some("<|eos|>"));
    }
}
