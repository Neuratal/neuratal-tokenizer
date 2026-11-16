//! Multilingual and domain-weighted corpus sampling and deterministic RNG.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Deterministic 64-bit pseudo-random number generator (XorShift64Star).
///
/// Implemented in 100% safe Rust without external dependencies for reproducible training.
#[derive(Debug, Clone)]
pub struct DeterministicRng {
    state: u64,
}

impl DeterministicRng {
    /// Creates a new PRNG seeded with an initial non-zero state.
    pub fn new(seed: u64) -> Self {
        let non_zero_seed = if seed == 0 { 0x853c49e6748fea9b } else { seed };
        Self {
            state: non_zero_seed,
        }
    }

    /// Generates next pseudo-random u64.
    #[inline(always)]
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }

    /// Generates a floating point number in [0.0, 1.0).
    #[inline(always)]
    pub fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0 / (1u64 << 53) as f64)
    }
}

/// A manifest entry describing an input corpus file and its sampling attributes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorpusManifestEntry {
    /// Path to the corpus file.
    pub path: PathBuf,
    /// Sampling weight multiplier (default: 1.0).
    #[serde(default = "default_weight")]
    pub weight: f64,
    /// Language code or tag (e.g., "en", "tr", "zh").
    pub language: Option<String>,
    /// Domain tag (e.g., "code", "math", "general").
    pub domain: Option<String>,
    /// Custom text field name for JSONL files (default: "text").
    pub text_field: Option<String>,
}

fn default_weight() -> f64 {
    1.0
}

/// Complete corpus training manifest.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CorpusManifest {
    pub entries: Vec<CorpusManifestEntry>,
    #[serde(default = "default_seed")]
    pub seed: u64,
}

fn default_seed() -> u64 {
    42
}

impl CorpusManifest {
    pub fn new(seed: u64) -> Self {
        Self {
            entries: Vec::new(),
            seed,
        }
    }

    pub fn add_entry(&mut self, entry: CorpusManifestEntry) {
        self.entries.push(entry);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deterministic_rng_reproducibility() {
        let mut rng1 = DeterministicRng::new(12345);
        let mut rng2 = DeterministicRng::new(12345);

        for _ in 0..100 {
            assert_eq!(rng1.next_u64(), rng2.next_u64());
            assert_eq!(rng1.next_f64(), rng2.next_f64());
        }
    }
}
