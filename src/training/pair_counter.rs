//! Array-backed doubly linked lists, inverted index, and priority-queue BPE merge engine.
//!
//! Highly optimized: Uses flat `Vec<Occurrence>` instead of expensive `HashSet` allocations,
//! with lazy validation during merge execution.

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

/// Doubly-linked list node representing a symbol in a word.
#[derive(Debug, Clone)]
pub struct SymbolNode {
    pub token_id: u32,
    pub prev: i32,
    pub next: i32,
}

/// A unique pre-token word with its associated corpus frequency.
#[derive(Debug, Clone)]
pub struct Word {
    pub count: usize,
    pub nodes: Vec<SymbolNode>,
}

/// Pointer to a pair occurrence within a specific word.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Occurrence {
    pub word_id: u32,
    pub node_idx: u32,
}

/// Heap entry for BPE pair frequency priority queue with deterministic tie-breaking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct HeapEntry {
    freq: u64,
    pair: (u32, u32),
}

impl Ord for HeapEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        // Primary: highest frequency first
        // Secondary: deterministic tie-breaker (smaller pair token IDs preferred)
        self.freq
            .cmp(&other.freq)
            .then_with(|| other.pair.cmp(&self.pair))
    }
}

impl PartialOrd for HeapEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// High-performance BPE merge trainer engine.
pub struct BpePairTrainer {
    words: Vec<Word>,
    pair_frequencies: HashMap<(u32, u32), u64>,
    pair_occurrences: HashMap<(u32, u32), Vec<Occurrence>>,
    priority_queue: BinaryHeap<HeapEntry>,
}

impl BpePairTrainer {
    /// Builds the trainer from unique pre-token word frequencies.
    pub fn from_word_counts(word_counts: HashMap<Vec<u8>, usize>) -> Self {
        let mut words = Vec::with_capacity(word_counts.len());
        let mut pair_frequencies: HashMap<(u32, u32), u64> = HashMap::new();
        let mut pair_occurrences: HashMap<(u32, u32), Vec<Occurrence>> = HashMap::new();

        // 1. Initialize words as array-backed doubly-linked lists.
        // Sort by byte content so internal word IDs and iteration order are
        // fully deterministic across runs, independent of HashMap state.
        let mut word_counts: Vec<(Vec<u8>, usize)> = word_counts.into_iter().collect();
        word_counts.sort_unstable_by(|a, b| a.0.cmp(&b.0));

        for (bytes, count) in word_counts {
            if bytes.is_empty() {
                continue;
            }

            let word_id = words.len() as u32;
            let mut nodes = Vec::with_capacity(bytes.len());

            for (i, &b) in bytes.iter().enumerate() {
                nodes.push(SymbolNode {
                    token_id: b as u32,
                    prev: if i > 0 { (i - 1) as i32 } else { -1 },
                    next: if i + 1 < bytes.len() {
                        (i + 1) as i32
                    } else {
                        -1
                    },
                });
            }

            // Index initial pairs
            for i in 0..nodes.len().saturating_sub(1) {
                let pair = (nodes[i].token_id, nodes[i + 1].token_id);
                *pair_frequencies.entry(pair).or_insert(0) += count as u64;
                pair_occurrences.entry(pair).or_default().push(Occurrence {
                    word_id,
                    node_idx: i as u32,
                });
            }

            words.push(Word { count, nodes });
        }

        // 2. Populate the priority queue
        let mut priority_queue = BinaryHeap::with_capacity(pair_frequencies.len());
        for (&pair, &freq) in &pair_frequencies {
            priority_queue.push(HeapEntry { freq, pair });
        }

        Self {
            words,
            pair_frequencies,
            pair_occurrences,
            priority_queue,
        }
    }

    /// Peeks at the best mergeable pair without applying it.
    ///
    /// Pops stale heap entries lazily, re-pushes the valid top entry, and returns it.
    /// Returns `None` when no pairs with `freq >= min_frequency` remain.
    pub fn peek_merge(&mut self, min_frequency: usize) -> Option<((u32, u32), u64)> {
        loop {
            let entry = self.priority_queue.pop()?;
            let current_freq = self.pair_frequencies.get(&entry.pair).copied().unwrap_or(0);
            if current_freq != entry.freq {
                continue; // discard stale heap entry
            }
            if current_freq < min_frequency as u64 {
                // Restore the valid entry for hygiene and stop: the heap is
                // ordered, so nothing better remains.
                self.priority_queue.push(entry);
                return None;
            }
            self.priority_queue.push(entry);
            return Some((entry.pair, current_freq));
        }
    }

    /// Removes a pair from consideration entirely (e.g. because its merged byte
    /// sequence already exists as a token, so materializing it would duplicate
    /// vocabulary entries). Queued heap entries for the pair become stale and are
    /// discarded lazily on pop.
    pub fn discard_pair(&mut self, pair: (u32, u32)) {
        self.pair_frequencies.remove(&pair);
        self.pair_occurrences.remove(&pair);
    }

    /// Applies the top mergeable pair. Must be called after a successful `peek_merge`;
    /// pops that exact entry, rewrites the affected words, and incrementally updates
    /// the inverted index and adjacent pairs.
    ///
    /// Returns the merged pair and its frequency, or `None` if the queue is empty.
    pub fn apply_merge(&mut self, next_token_id: u32) -> Option<((u32, u32), u64)> {
        // Pop top pair with lazy invalidation
        let (best_pair, freq) = loop {
            let entry = self.priority_queue.pop()?;
            let current_freq = self.pair_frequencies.get(&entry.pair).copied().unwrap_or(0);

            if current_freq == entry.freq {
                break (entry.pair, current_freq);
            }
            // Discard stale entry and continue
        };

        let (left_token, right_token) = best_pair;
        let occurrences = self.pair_occurrences.remove(&best_pair).unwrap_or_default();
        self.pair_frequencies.remove(&best_pair);

        // Process all occurrences and incrementally update inverted index and adjacent pairs
        for occ in occurrences {
            let word = &mut self.words[occ.word_id as usize];
            let left_idx = occ.node_idx as usize;

            // Validate that the pair is still intact
            if left_idx >= word.nodes.len() {
                continue;
            }
            if word.nodes[left_idx].token_id != left_token {
                continue;
            }
            let right_idx_i32 = word.nodes[left_idx].next;
            if right_idx_i32 == -1 {
                continue;
            }
            let right_idx = right_idx_i32 as usize;
            if word.nodes[right_idx].token_id != right_token {
                continue;
            }

            let word_count = word.count as u64;
            let prev_idx = word.nodes[left_idx].prev;
            let next_idx = word.nodes[right_idx].next;

            // 1. Decrement old left-adjacent pair: (P, left)
            if prev_idx != -1 {
                let p_id = word.nodes[prev_idx as usize].token_id;
                let old_left_pair = (p_id, left_token);
                if let Some(f) = self.pair_frequencies.get_mut(&old_left_pair) {
                    *f = f.saturating_sub(word_count);
                    let updated_freq = *f;
                    if updated_freq == 0 {
                        self.pair_frequencies.remove(&old_left_pair);
                        // No valid occurrences remain; release the occurrence list.
                        self.pair_occurrences.remove(&old_left_pair);
                    } else {
                        // Push the updated frequency so the pair remains selectable.
                        // Stale higher-frequency entries are discarded lazily on pop.
                        self.priority_queue.push(HeapEntry {
                            freq: updated_freq,
                            pair: old_left_pair,
                        });
                    }
                }
            }

            // 2. Decrement old right-adjacent pair: (right, S)
            if next_idx != -1 {
                let s_id = word.nodes[next_idx as usize].token_id;
                let old_right_pair = (right_token, s_id);
                if let Some(f) = self.pair_frequencies.get_mut(&old_right_pair) {
                    *f = f.saturating_sub(word_count);
                    let updated_freq = *f;
                    if updated_freq == 0 {
                        self.pair_frequencies.remove(&old_right_pair);
                        self.pair_occurrences.remove(&old_right_pair);
                    } else {
                        self.priority_queue.push(HeapEntry {
                            freq: updated_freq,
                            pair: old_right_pair,
                        });
                    }
                }
            }

            // 3. Execute the merge: left node becomes new_token_id, bypass right node
            word.nodes[left_idx].token_id = next_token_id;
            word.nodes[left_idx].next = next_idx;
            // Mark the consumed right node as dead so that overlapping pair
            // occurrences in the same step are invalidated correctly.
            word.nodes[right_idx].token_id = u32::MAX;
            if next_idx != -1 {
                word.nodes[next_idx as usize].prev = left_idx as i32;
            }

            // 4. Increment new left-adjacent pair: (P, new_token_id)
            if prev_idx != -1 {
                let p_id = word.nodes[prev_idx as usize].token_id;
                let new_left_pair = (p_id, next_token_id);
                let new_f = self.pair_frequencies.entry(new_left_pair).or_insert(0);
                *new_f += word_count;
                let updated_freq = *new_f;
                self.pair_occurrences
                    .entry(new_left_pair)
                    .or_default()
                    .push(Occurrence {
                        word_id: occ.word_id,
                        node_idx: prev_idx as u32,
                    });
                self.priority_queue.push(HeapEntry {
                    freq: updated_freq,
                    pair: new_left_pair,
                });
            }

            // 5. Increment new right-adjacent pair: (new_token_id, S)
            if next_idx != -1 {
                let s_id = word.nodes[next_idx as usize].token_id;
                let new_right_pair = (next_token_id, s_id);
                let new_f = self.pair_frequencies.entry(new_right_pair).or_insert(0);
                *new_f += word_count;
                let updated_freq = *new_f;
                self.pair_occurrences
                    .entry(new_right_pair)
                    .or_default()
                    .push(Occurrence {
                        word_id: occ.word_id,
                        node_idx: left_idx as u32,
                    });
                self.priority_queue.push(HeapEntry {
                    freq: updated_freq,
                    pair: new_right_pair,
                });
            }
        }

        Some((best_pair, freq))
    }

    /// Selects and executes the best BPE merge step.
    ///
    /// Returns:
    /// - `Some((best_pair, frequency))` when a valid merge is found and executed.
    /// - `None` when no pairs with `freq >= min_frequency` remain.
    ///
    /// Convenience wrapper around `peek_merge` + `apply_merge`; callers that need
    /// to inspect the candidate (e.g. to reject byte-sequence duplicates) should use
    /// those two functions directly.
    pub fn step_merge(
        &mut self,
        next_token_id: u32,
        min_frequency: usize,
    ) -> Option<((u32, u32), u64)> {
        self.peek_merge(min_frequency)?;
        self.apply_merge(next_token_id)
    }

    /// Returns the number of unique words being tracked.
    pub fn num_words(&self) -> usize {
        self.words.len()
    }

    /// Returns the count of distinct active pair frequency entries.
    pub fn pair_frequencies_count(&self) -> usize {
        self.pair_frequencies.len()
    }

    /// Returns the total number of pair occurrence entries tracked.
    pub fn total_occurrences_count(&self) -> usize {
        self.pair_occurrences.values().map(|v| v.len()).sum()
    }

    /// Returns the current size of the priority queue (including lazily invalid entries).
    pub fn priority_queue_size(&self) -> usize {
        self.priority_queue.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bpe_pair_trainer_basic() {
        let mut counts = HashMap::new();
        counts.insert(b"th".to_vec(), 2);
        counts.insert(b"the".to_vec(), 5);
        counts.insert(b"then".to_vec(), 3);

        let mut trainer = BpePairTrainer::from_word_counts(counts);
        assert_eq!(trainer.num_words(), 3);

        let (first_pair, freq) = trainer.step_merge(256, 1).expect("Merge step failed");
        assert_eq!(first_pair, (b't' as u32, b'h' as u32));
        assert_eq!(freq, 10);

        let (second_pair, freq2) = trainer.step_merge(257, 1).expect("Second merge failed");
        assert_eq!(second_pair, (256, b'e' as u32));
        assert_eq!(freq2, 8);
    }

    /// Regression test: a pair whose frequency is *decremented* (but stays above
    /// min_frequency) must remain selectable in later merge steps.
    ///
    /// Corpus (unique words -> counts):
    ///   "abc" x6, "abd" x6, "bc" x13
    /// Initial: (a,b)=12, (b,c)=19, (b,d)=6.
    /// After merging (b,c)@19, (a,b) drops to 6 without any increment, so without
    /// re-queuing the decremented count the pair would vanish from the queue.
    #[test]
    fn test_decremented_pair_remains_selectable() {
        let mut counts = HashMap::new();
        counts.insert(b"abc".to_vec(), 6);
        counts.insert(b"abd".to_vec(), 6);
        counts.insert(b"bc".to_vec(), 13);

        let mut trainer = BpePairTrainer::from_word_counts(counts);

        let (first_pair, first_freq) = trainer.step_merge(256, 2).expect("first merge");
        assert_eq!(first_pair, (b'b' as u32, b'c' as u32));
        assert_eq!(first_freq, 19);

        let (second_pair, _second_freq) = trainer.step_merge(257, 2).expect("second merge");
        assert_eq!(
            second_pair,
            (b'a' as u32, b'b' as u32),
            "decremented pair (a,b) must still be selected; got {:?}",
            second_pair
        );
    }
}
