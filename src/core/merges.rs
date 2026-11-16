//! BPE Merge table representation and lookup.
//!
//! Stores ordered BPE merge operations. Each merge combines a pair of existing
//! tokens (left, right) into a new token ID, with a rank determining its priority.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};

/// Multiplicative integer hasher (fxhash-style) for `(u32, u32)` merge pairs.
/// SipHash is overkill for 8-byte integer keys on the encode hot path;
/// this keeps lookups correct while being measurably cheaper.
#[derive(Default)]
struct FxHasher {
    hash: u64,
}

impl Hasher for FxHasher {
    #[inline]
    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.add(b as u64);
        }
    }
    #[inline]
    fn write_u32(&mut self, x: u32) {
        self.add(x as u64);
    }
    #[inline]
    fn write_u64(&mut self, x: u64) {
        self.add(x);
    }
    #[inline]
    fn finish(&self) -> u64 {
        self.hash
    }
}

impl FxHasher {
    #[inline]
    fn add(&mut self, i: u64) {
        const SEED: u64 = 0x51_7c_c1_b7_27_22_0a_95;
        self.hash = (self.hash.rotate_left(5) ^ i).wrapping_mul(SEED);
    }
}

type PairBuildHasher = BuildHasherDefault<FxHasher>;

/// Represents an ordered merge rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MergeRule {
    pub left: u32,
    pub right: u32,
    pub target: u32,
    pub rank: u32,
}

/// Table storing all learned BPE merges.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeTable {
    /// Fast lookup from pair (left, right) to (target_id, rank).
    pair_to_rule: HashMap<(u32, u32), (u32, u32), PairBuildHasher>,
    /// Ordered vector of merge rules, indexed by rank.
    rules: Vec<MergeRule>,
}

#[derive(Serialize, Deserialize)]
struct MergeTableDef {
    rules: Vec<MergeRule>,
}

impl Serialize for MergeTable {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        MergeTableDef {
            rules: self.rules.clone(),
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for MergeTable {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let def = MergeTableDef::deserialize(deserializer)?;
        let mut pair_to_rule = HashMap::with_capacity_and_hasher(def.rules.len(), PairBuildHasher::default());
        for rule in &def.rules {
            pair_to_rule.insert((rule.left, rule.right), (rule.target, rule.rank));
        }
        Ok(Self {
            pair_to_rule,
            rules: def.rules,
        })
    }
}

impl Default for MergeTable {
    fn default() -> Self {
        Self::new()
    }
}

impl MergeTable {
    /// Creates an empty merge table.
    pub fn new() -> Self {
        Self {
            pair_to_rule: HashMap::with_hasher(PairBuildHasher::default()),
            rules: Vec::new(),
        }
    }

    /// Adds a merge rule to the table.
    pub fn add_merge(&mut self, left: u32, right: u32, target: u32) -> Result<u32, String> {
        if target < 256 {
            return Err(format!(
                "Cannot create merge target ID {}: IDs 0..=255 are strictly reserved for base bytes.",
                target
            ));
        }
        if let Some(&(existing_target, existing_rank)) = self.pair_to_rule.get(&(left, right)) {
            return Err(format!(
                "Pair ({}, {}) already merged into ID {} at rank {}",
                left, right, existing_target, existing_rank
            ));
        }

        let rank = self.rules.len() as u32;
        self.pair_to_rule.insert((left, right), (target, rank));
        self.rules.push(MergeRule {
            left,
            right,
            target,
            rank,
        });

        Ok(rank)
    }

    /// Looks up the merge rule for a pair, returning `Some((target_id, rank))`.
    #[inline(always)]
    pub fn get_merge(&self, left: u32, right: u32) -> Option<(u32, u32)> {
        self.pair_to_rule.get(&(left, right)).copied()
    }

    /// Looks up the priority rank for a pair.
    #[inline(always)]
    pub fn get_rank(&self, left: u32, right: u32) -> Option<u32> {
        self.pair_to_rule.get(&(left, right)).map(|&(_, rank)| rank)
    }

    /// Returns the total number of merges.
    pub fn len(&self) -> usize {
        self.rules.len()
    }

    /// Checks if the merge table is empty.
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    /// Returns all merge rules ordered by rank.
    pub fn rules(&self) -> &[MergeRule] {
        &self.rules
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merges_require_id_above_255() {
        let mut table = MergeTable::new();
        assert!(table.add_merge(b'h' as u32, b'e' as u32, 200).is_err());
        assert!(table.add_merge(b'h' as u32, b'e' as u32, 256).is_ok());
    }

    #[test]
    fn test_merges_rank_and_lookup() {
        let mut table = MergeTable::new();
        let rank0 = table.add_merge(b't' as u32, b'h' as u32, 256).unwrap();
        let rank1 = table.add_merge(256, b'e' as u32, 257).unwrap();

        assert_eq!(rank0, 0);
        assert_eq!(rank1, 1);
        assert_eq!(table.get_merge(b't' as u32, b'h' as u32), Some((256, 0)));
        assert_eq!(table.get_merge(256, b'e' as u32), Some((257, 1)));
        assert_eq!(table.get_merge(b'e' as u32, 256), None);
    }

    #[test]
    fn test_merges_serde_roundtrip() {
        let mut table = MergeTable::new();
        table.add_merge(b't' as u32, b'h' as u32, 256).unwrap();
        table.add_merge(256, b'e' as u32, 257).unwrap();

        let json = serde_json::to_string(&table).unwrap();
        let loaded: MergeTable = serde_json::from_str(&json).unwrap();

        assert_eq!(loaded.len(), table.len());
        assert_eq!(loaded.get_merge(b't' as u32, b'h' as u32), Some((256, 0)));
        assert_eq!(loaded.get_merge(256, b'e' as u32), Some((257, 1)));
    }
}
