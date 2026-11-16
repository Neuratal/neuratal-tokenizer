//! Deterministic byte-level BPE encoder.
//!
//! Provides two independent encoding APIs:
//! - `encode_bytes`: Encodes arbitrary raw byte slices (supports invalid UTF-8).
//! - `encode_text`: Encodes valid UTF-8 strings with special token handling and pre-tokenization.

use crate::core::merges::MergeTable;
use crate::core::special_tokens::{AllowedSpecial, SpecialTokenRegistry};
use crate::core::vocabulary::TokenId;
use crate::encoding::normalizer::Normalization;
use crate::encoding::pretokenizer::PretokenizationMode;
use std::fmt;

/// Errors that can occur during tokenization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenizerError {
    InvalidUtf8(String),
    SpecialTokenError(String),
    EncodingError(String),
    DecodingError(String),
}

impl fmt::Display for TokenizerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenizerError::InvalidUtf8(msg) => write!(f, "Invalid UTF-8: {}", msg),
            TokenizerError::SpecialTokenError(msg) => write!(f, "Special token error: {}", msg),
            TokenizerError::EncodingError(msg) => write!(f, "Encoding error: {}", msg),
            TokenizerError::DecodingError(msg) => write!(f, "Decoding error: {}", msg),
        }
    }
}

impl std::error::Error for TokenizerError {}

/// Configuration options for the encoding process.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncodeOptions {
    /// Strategy for recognizing special tokens in text.
    pub allowed_special: AllowedSpecial,
    /// Normalization form to apply (default: None).
    pub normalization: Normalization,
    /// Pre-tokenization strategy mode.
    pub pretokenizer: PretokenizationMode,
}

impl Default for EncodeOptions {
    fn default() -> Self {
        Self {
            allowed_special: AllowedSpecial::All,
            normalization: Normalization::None,
            pretokenizer: PretokenizationMode::default(),
        }
    }
}

/// Doubly-linked list node for fast priority BPE merges within a single pre-token slice.
#[derive(Debug, Clone)]
struct BpeNode {
    id: TokenId,
    prev: i32,
    next: i32,
    /// False once the node has been absorbed as the right-hand side of a merge.
    alive: bool,
}

/// Min-priority queue entry: lowest merge rank first, then leftmost position.
/// Entries are validated lazily; stale entries are discarded on pop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MergeCandidate {
    rank: u32,
    position: i32,
    target: TokenId,
}

impl Ord for MergeCandidate {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // BinaryHeap is a max-heap; invert so smaller rank/position pops first.
        other
            .rank
            .cmp(&self.rank)
            .then_with(|| other.position.cmp(&self.position))
    }
}

impl PartialOrd for MergeCandidate {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// Encodes an arbitrary byte slice into token IDs.
///
/// This function never fails and supports arbitrary binary data, including invalid UTF-8.
pub fn encode_bytes(
    bytes: &[u8],
    merges: &MergeTable,
    pretokenizer: &PretokenizationMode,
) -> Vec<TokenId> {
    if bytes.is_empty() {
        return Vec::new();
    }

    let slices = pretokenizer.split_bytes(bytes);
    let mut all_tokens = Vec::with_capacity(bytes.len());
    let mut scratch = EncodeScratch::new();

    for slice in slices {
        encode_single_slice(slice, merges, &mut all_tokens, &mut scratch);
    }

    all_tokens
}

/// Encodes a UTF-8 text string into token IDs.
///
/// Handles special token isolation (preventing special tokens from being split or merged with ordinary text),
/// text normalization, and pre-tokenization.
pub fn encode_text(
    text: &str,
    merges: &MergeTable,
    special_registry: &SpecialTokenRegistry,
    options: &EncodeOptions,
) -> Result<Vec<TokenId>, TokenizerError> {
    if text.is_empty() {
        return Ok(Vec::new());
    }

    let mut output_tokens = Vec::with_capacity(text.len() / 2 + 1);
    let mut scratch = EncodeScratch::new();

    // Split text into special token matches and plain text segments
    let segments = special_registry.split_text(text, &options.allowed_special);

    for (start, end, special_id) in segments {
        if let Some(id) = special_id {
            // Emitted directly as a special token
            output_tokens.push(id);
        } else {
            let plain_str = &text[start..end];
            let normalized = options.normalization.normalize(plain_str);
            let slices = options.pretokenizer.split_text(&normalized);

            for slice in slices {
                encode_single_slice(slice.as_bytes(), merges, &mut output_tokens, &mut scratch);
            }
        }
    }

    Ok(output_tokens)
}

/// Reusable buffers for the BPE loop: creating freshly-allocated node vecs and
/// heaps per pre-token generated measurable allocator churn. The scratch is
/// created once per top-level encode call and reused for every slice.
struct EncodeScratch {
    nodes: Vec<BpeNode>,
    heap: std::collections::BinaryHeap<MergeCandidate>,
}

impl EncodeScratch {
    fn new() -> Self {
        Self {
            nodes: Vec::new(),
            heap: std::collections::BinaryHeap::new(),
        }
    }
}

/// Core BPE merge loop for an individual pre-token byte slice.
fn encode_single_slice(
    slice: &[u8],
    merges: &MergeTable,
    output: &mut Vec<TokenId>,
    scratch: &mut EncodeScratch,
) {
    let len = slice.len();
    if len == 0 {
        return;
    }
    if len == 1 {
        output.push(slice[0] as TokenId);
        return;
    }

    // Build array-backed doubly-linked list of base byte tokens
    let EncodeScratch { nodes, heap } = scratch;
    nodes.clear();
    nodes.extend(slice.iter().enumerate().map(|(i, &b)| BpeNode {
        id: b as TokenId,
        prev: if i > 0 { (i - 1) as i32 } else { -1 },
        next: if i + 1 < len { (i + 1) as i32 } else { -1 },
        alive: true,
    }));

    if merges.is_empty() {
        for node in nodes.iter() {
            output.push(node.id);
        }
        return;
    }

    // Seed the priority queue with every currently mergeable adjacent pair.
    heap.clear();
    for i in 0..nodes.len().saturating_sub(1) {
        if let Some((target, rank)) = merges.get_merge(nodes[i].id, nodes[i + 1].id) {
            heap.push(MergeCandidate {
                rank,
                position: i as i32,
                target,
            });
        }
    }

    while let Some(candidate) = heap.pop() {
        let left_idx = candidate.position as usize;
        // Lazily validate: both nodes must be alive and must still form the
        // exact pair this candidate was created for.
        if !nodes[left_idx].alive {
            continue;
        }
        let right_idx_i32 = nodes[left_idx].next;
        if right_idx_i32 == -1 {
            continue;
        }
        let right_idx = right_idx_i32 as usize;
        if !nodes[right_idx].alive {
            continue;
        }
        match merges.get_merge(nodes[left_idx].id, nodes[right_idx].id) {
            Some((target, rank)) if target == candidate.target && rank == candidate.rank => {}
            _ => continue, // stale entry
        }

        // Perform the merge: replace left node with target_id, bypass right node
        let target_id = candidate.target;
        nodes[left_idx].id = target_id;
        nodes[left_idx].next = nodes[right_idx].next;
        nodes[right_idx].alive = false;
        let after_right = nodes[right_idx].next;
        if after_right != -1 {
            nodes[after_right as usize].prev = left_idx as i32;
        }

        // Re-check the pairs that became adjacent to the merged node.
        let prev_idx = nodes[left_idx].prev;
        if prev_idx != -1 {
            let p = prev_idx as usize;
            if let Some((target, rank)) = merges.get_merge(nodes[p].id, nodes[left_idx].id) {
                heap.push(MergeCandidate {
                    rank,
                    position: prev_idx,
                    target,
                });
            }
        }
        if nodes[left_idx].next != -1 {
            let n = nodes[left_idx].next as usize;
            if let Some((target, rank)) = merges.get_merge(nodes[left_idx].id, nodes[n].id) {
                heap.push(MergeCandidate {
                    rank,
                    position: left_idx as i32,
                    target,
                });
            }
        }
    }

    // Collect surviving token IDs in sequence
    let mut idx = 0i32;
    while idx != -1 {
        output.push(nodes[idx as usize].id);
        idx = nodes[idx as usize].next;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_bytes_base() {
        let merges = MergeTable::new();
        let bytes = b"hello";
        let tokens = encode_bytes(bytes, &merges, &PretokenizationMode::RawByte);
        assert_eq!(
            tokens,
            vec![
                b'h' as u32,
                b'e' as u32,
                b'l' as u32,
                b'l' as u32,
                b'o' as u32
            ]
        );
    }

    #[test]
    fn test_encode_with_merges() {
        let mut merges = MergeTable::new();
        // Merge 'l' + 'l' -> 256
        merges.add_merge(b'l' as u32, b'l' as u32, 256).unwrap();
        // Merge 'h' + 'e' -> 257
        merges.add_merge(b'h' as u32, b'e' as u32, 257).unwrap();

        let bytes = b"hello";
        let tokens = encode_bytes(bytes, &merges, &PretokenizationMode::RawByte);
        assert_eq!(tokens, vec![257, 256, b'o' as u32]);
    }

    #[test]
    fn test_encode_invalid_utf8_binary() {
        let merges = MergeTable::new();
        let invalid_bytes = &[0xFF, 0xFE, 0x80, 0x00];
        let tokens = encode_bytes(invalid_bytes, &merges, &PretokenizationMode::RawByte);
        assert_eq!(tokens, vec![255, 254, 128, 0]);
    }
}
