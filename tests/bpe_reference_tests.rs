//! Independent reference BPE implementation (test-only, intentionally simple/slow).
//! Used to differentially validate the optimized production trainer.

use std::collections::HashMap;

/// Encode raw (valid UTF-8) bytes as a JSON string body with required escapes.
fn json_escape_bytes(word: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(word.len() + 8);
    for &b in word {
        match b {
            b'\\' => out.extend_from_slice(b"\\\\"),
            b'"' => out.extend_from_slice(b"\\\""),
            0x00..=0x1F => out.extend_from_slice(format!("\\u{:04x}", b).as_bytes()),
            _ => out.push(b),
        }
    }
    out
}

/// Build one JSONL line {"text": "..."} for the given word bytes.
fn jsonl_line(word: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(word.len() + 12);
    out.extend_from_slice(b"{\"text\": \"");
    out.extend_from_slice(&json_escape_bytes(word));
    out.extend_from_slice(b"\"}\n");
    out
}

#[derive(Debug, Clone)]
pub struct ReferenceModel {
    /// token bytes for each id: 0..=255 base bytes, then merged tokens in rank order.
    pub id_to_bytes: Vec<Vec<u8>>,
    /// merge rules in rank order: (left_id, right_id, target_id)
    pub rules: Vec<(u32, u32, u32)>,
}

/// Reference corpus for differential testing: tricky multilingual/code/digit words
/// with repeated substructures.
fn reference_word_counts() -> HashMap<Vec<u8>, usize> {
    let mut counts = HashMap::new();
    let words: &[(&[u8], usize)] = &[
        (b"the", 500),
        (b"there", 120),
        (b"then", 80),
        (b"cat", 60),
        (b"concatenate", 40),
        (b"aa", 50),
        (b"aab", 45),
        (b"abba", 30),
        (b"aabb", 25),
        (b"bb", 100),
        (b"abcd", 35),
        (b"abcabc", 60),
        (b"function", 90),
        (b"return", 70),
        (b"class", 65),
        (b"print", 55),
        (b"eval", 20),
        (b"true", 48),
        (b"false", 40),
        (b"null", 38),
        (b"import", 44),
        (b"export", 42),
        (b"const", 58),
        (b"let", 36),
        (b"var", 30),
        (b"if", 77),
        (b"else", 61),
        (b"while", 29),
        (b"for", 85),
        (b"in", 150),
        (b"to", 200),
        (b"of", 180),
        (b"a", 400),
        (b"an", 100),
        (b"and", 90),
        (b"or", 70),
        (b"not", 66),
        (b"is", 111),
        (b"it", 95),
        (b"hello", 75),
        (b"world", 72),
        (b"foo", 25),
        (b"bar", 22),
        (b"baz", 20),
        (b"qux", 18),
        (b"123", 80),
        (b"456", 40),
        (b"1.5e3", 15),
        (b"2026", 35),
        (b"12", 60),
        (b"345", 12),
        (b"abc", 210),
        (b"def", 140),
        (b"ghi", 70),
        (b"abcabcabc", 200),
        (b"threshold", 33),
        (b"therapist", 15),
        ("Türkçe".as_bytes(), 90),
        ("İstanbul".as_bytes(), 30),
        ("masaüstü".as_bytes(), 25),
        ("العربية".as_bytes(), 50),
        ("中文测试".as_bytes(), 40),
        ("Ελληνικά".as_bytes(), 35),
    ];
    for (w, c) in words {
        *counts.entry(w.to_vec()).or_insert(0) += c;
    }
    counts
}

#[test]
fn reference_trainer_produces_expected_merges() {
    let counts = reference_word_counts();
    let model = reference_train(&counts, 80, 2);
    assert!(!model.rules.is_empty(), "reference should learn merges");
    assert_eq!(model.id_to_bytes.len(), 256 + model.rules.len());
    for word in counts.keys() {
        let ids = reference_encode(word, &model);
        let decoded = reference_decode(&ids, &model);
        assert_eq!(&decoded, word, "reference roundtrip failed for {:?}", word);
    }
}

#[test]
fn production_matches_reference_on_word_level_corpus() {
    use neuratal_tokenizer::*;

    let counts = reference_word_counts();

    // Write the exact same unique-words corpus to a temp file and train production.
    let corpus_path = std::env::temp_dir().join(format!("diff_corpus_{}.jsonl", std::process::id()));
    {
        use std::io::Write;
        let mut f = std::fs::File::create(&corpus_path).unwrap();
        for (word, count) in &counts {
            for _ in 0..*count {
                f.write_all(&jsonl_line(word)).unwrap();
            }
        }
    }

    let mut manifest = CorpusManifest::new(42);
    manifest.add_entry(CorpusManifestEntry {
        path: corpus_path.clone(),
        weight: 1.0,
        language: None,
        domain: None,
        text_field: None,
    });

    // Production trains on whitespace-separated pre-tokens; use RawByte to keep
    // the same segmentation as the reference (word-level).
    let config = TrainingConfig {
        target_vocab_size: 256 + 40 + 2, // 40 merges + 2 special tokens
        min_frequency: 2,
        pretokenizer: PretokenizationMode::RawByte,
        normalization: Normalization::None,
        special_tokens: vec!["<|bos|>".to_string(), "<|eos|>".to_string()],
        seed: 42,
    };

    let (prod_tok, _stats) = train_from_manifest(&manifest, &config).unwrap();

    let reference = reference_train(&counts, 40, 2);

    // Compare merge sequences: rank, left, right, target.
    let prod_rules = prod_tok.merges().rules();
    assert_eq!(
        prod_rules.len(),
        reference.rules.len(),
        "production learned {} merges, reference learned {}",
        prod_rules.len(),
        reference.rules.len()
    );
    for (i, rule) in prod_rules.iter().enumerate() {
        let (l, r, t) = reference.rules[i];
        assert_eq!(
            (rule.left, rule.right, rule.target),
            (l, r, t),
            "merge rank {} differs: prod=({}, {}, {}), ref=({}, {}, {})",
            i,
            rule.left,
            rule.right,
            rule.target,
            l,
            r,
            t
        );
    }

    // Compare vocab bytes
    for id in 0..(256 + prod_rules.len()) {
        assert_eq!(
            prod_tok.vocab().get_bytes(id as u32),
            Some(reference.id_to_bytes[id].as_slice()),
            "vocab bytes differ at id {}",
            id
        );
    }

    // Encode/decode round-trip on raw text through production model
    let text = "the cat concatenated the 123 things in 2026";
    let ids = prod_tok.encode_text(text).unwrap();
    let decoded = prod_tok.decode_text(&ids).unwrap();
    assert_eq!(decoded, text);

    let _ = std::fs::remove_file(corpus_path);
}

#[test]
fn heavy_repeated_pair_overlap_is_handled() {
    // The production engine must merge in an occurrence with overlapping positions correctly.
    use neuratal_tokenizer::*;
    let corpus_path =
        std::env::temp_dir().join(format!("overlap_corpus_{}.jsonl", std::process::id()));
    {
        use std::io::Write;
        let mut f = std::fs::File::create(&corpus_path).unwrap();
        // "aaa" contains pairs (a,a) twice, overlapping at positions 0-1 and 1-2.
        // The merge of (a,a)->X must consume BOTH occurrences by counting the
        // word count twice for a single word occurrence, which tests the
        // word_count handling in pair_counter.
        for _ in 0..10 {
            f.write_all(&jsonl_line(b"aaaaaaaa")).unwrap();
        }
        // "aabb" adds another (a,a) + (a,b) combination
        for _ in 0..7 {
            f.write_all(&jsonl_line(b"aabb")).unwrap();
        }
    }

    let mut manifest = CorpusManifest::new(42);
    manifest.add_entry(CorpusManifestEntry {
        path: corpus_path.clone(),
        weight: 1.0,
        language: None,
        domain: None,
        text_field: None,
    });

    let config = TrainingConfig {
        target_vocab_size: 262, // 256 base + 4 merges + 2 special
        min_frequency: 2,
        pretokenizer: PretokenizationMode::RawByte,
        normalization: Normalization::None,
        special_tokens: vec!["<|bos|>".to_string(), "<|eos|>".to_string()],
        seed: 42,
    };

    let (tok, stats) = train_from_manifest(&manifest, &config).unwrap();
    assert_eq!(stats.merges_learned, 4);
    assert_eq!(tok.vocab_size(), 262);

    let text = "aaaaaaaa aabb";
    let ids = tok.encode_text(text).unwrap();
    let decoded = tok.decode_text(&ids).unwrap();
    assert_eq!(decoded, text);

    let _ = std::fs::remove_file(corpus_path);
}

/// Naive reference word-level BPE training over unique word frequencies.
pub fn reference_train(
    word_counts: &HashMap<Vec<u8>, usize>,
    target_merges: usize,
    min_frequency: usize,
) -> ReferenceModel {
    let mut id_to_bytes: Vec<Vec<u8>> = (0u8..=255u8).map(|b| vec![b]).collect();
    let mut rules: Vec<(u32, u32, u32)> = Vec::new();

    let mut words: Vec<Vec<u32>> = word_counts
        .keys()
        .map(|w| w.iter().map(|&b| b as u32).collect())
        .collect();
    let counts: Vec<usize> = word_counts.keys().map(|k| word_counts[k]).collect();

    let mut banned: Vec<(u32, u32)> = Vec::new();

    for _ in 0..target_merges {
        loop {
            let mut freq: HashMap<(u32, u32), u64> = HashMap::new();
            for (wi, word) in words.iter().enumerate() {
                if word.len() < 2 {
                    continue;
                }
                let c = counts[wi] as u64;
                for pair in word.windows(2) {
                    *freq.entry((pair[0], pair[1])).or_insert(0) += c;
                }
            }

            // Drop banned pairs (duplicate-byte candidates rejected like production).
            for pair in &banned {
                freq.remove(pair);
            }

            if freq.is_empty() {
                return ReferenceModel { id_to_bytes, rules };
            }

            let (pair, pair_freq) = freq
                .iter()
                .max_by_key(|&(&p, &f)| (f, std::cmp::Reverse(p)))
                .map(|(&p, &f)| (p, f))
                .unwrap();

            if pair_freq < min_frequency as u64 {
                return ReferenceModel { id_to_bytes, rules };
            }

            let mut candidate_bytes = id_to_bytes[pair.0 as usize].clone();
            candidate_bytes.extend_from_slice(&id_to_bytes[pair.1 as usize]);
            if id_to_bytes.iter().any(|b| b == &candidate_bytes) {
                banned.push(pair);
                continue; // try next best candidate
            }

            // Merge the pair everywhere.
            let target_id = id_to_bytes.len() as u32;
            id_to_bytes.push(candidate_bytes);
            rules.push((pair.0, pair.1, target_id));

            for word in words.iter_mut() {
                let mut i = 0;
                while i + 1 < word.len() {
                    if word[i] == pair.0 && word[i + 1] == pair.1 {
                        word.splice(i..i + 2, [target_id]);
                        i += 1;
                    } else {
                        i += 1;
                    }
                }
            }
            break;
        }
    }

    ReferenceModel { id_to_bytes, rules }
}

/// Naive encode with the reference model.
pub fn reference_encode(bytes: &[u8], model: &ReferenceModel) -> Vec<u32> {
    // Map pair -> (rank, target)
    let mut seq: Vec<u32> = bytes.iter().map(|&b| b as u32).collect();
    loop {
        let mut best: Option<(usize, u32, u32)> = None; // (index, rank, target)
        for i in 0..seq.len().saturating_sub(1) {
            if let Some((rank, target)) = find_rule(&model.rules, seq[i], seq[i + 1]) {
                match best {
                    None => best = Some((i, rank, target)),
                    Some((_, r, _)) if rank < r => best = Some((i, rank, target)),
                    _ => {}
                }
            }
        }
        match best {
            None => return seq,
            Some((i, _, target)) => {
                seq.splice(i..i + 2, [target]);
            }
        }
    }
}

fn find_rule(rules: &[(u32, u32, u32)], left: u32, right: u32) -> Option<(u32, u32)> {
    for (i, &(l, r, t)) in rules.iter().enumerate() {
        if l == left && r == right {
            return Some((i as u32, t));
        }
    }
    None
}

/// Naive decode: concatenate token byte sequences.
pub fn reference_decode(ids: &[u32], model: &ReferenceModel) -> Vec<u8> {
    let mut out = Vec::new();
    for &id in ids {
        out.extend_from_slice(&model.id_to_bytes[id as usize]);
    }
    out
}

// ---------------------------------------------------------------------------
// Extended differential coverage (release verification)
// ---------------------------------------------------------------------------

use neuratal_tokenizer::*;
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

static TEMP_SEQ: AtomicUsize = AtomicUsize::new(0);

fn temp_corpus_for(label: &str, counts: &HashMap<Vec<u8>, usize>) -> PathBuf {
    let seq = TEMP_SEQ.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "ntok_diff_{}_{}_{}.jsonl",
        std::process::id(),
        seq,
        label
    ));
    let mut f = std::fs::File::create(&path).unwrap();
    for (word, count) in counts {
        assert!(!word.is_empty());
        for _ in 0..*count {
            f.write_all(&jsonl_line(word)).unwrap();
        }
    }
    f.flush().unwrap();
    path
}

fn counts(pairs: &[(&[u8], usize)]) -> HashMap<Vec<u8>, usize> {
    let mut map = HashMap::new();
    for (word, count) in pairs {
        *map.entry(word.to_vec()).or_insert(0) += *count;
    }
    map
}

/// Train production on the same word-frequency corpus as the reference and
/// compare the complete merge history, vocabulary bytes, and round-trip.
fn differential_check(
    label: &str,
    word_counts: &HashMap<Vec<u8>, usize>,
    target_merges: usize,
    min_frequency: usize,
) {
    let path = temp_corpus_for(label, word_counts);
    let reference = reference_train(word_counts, target_merges, min_frequency);

    let mut manifest = CorpusManifest::new(42);
    manifest.add_entry(CorpusManifestEntry {
        path: path.clone(),
        weight: 1.0,
        language: None,
        domain: None,
        text_field: None,
    });
    let config = TrainingConfig {
        target_vocab_size: 256 + target_merges + 2,
        min_frequency,
        pretokenizer: PretokenizationMode::RawByte,
        normalization: Normalization::None,
        special_tokens: vec!["<|bos|>".to_string(), "<|eos|".to_string()],
        seed: 42,
    };
    let (prod, stats) = train_from_manifest(&manifest, &config).unwrap();
    let prod_rules = prod.merges().rules();

    assert_eq!(
        stats.unique_words_count,
        word_counts.len(),
        "{label}: unique word count differs"
    );
    assert_eq!(
        stats.merges_learned,
        reference.rules.len(),
        "{label}: merge count differs (prod {} vs ref {})",
        stats.merges_learned,
        reference.rules.len()
    );
    assert_eq!(
        prod_rules.len(),
        reference.rules.len(),
        "{label}: merge table length differs"
    );
    for (i, rule) in prod_rules.iter().enumerate() {
        let (l, r, t) = reference.rules[i];
        assert_eq!(
            (rule.left, rule.right, rule.target, rule.rank),
            (l, r, t, i as u32),
            "{label}: merge at rank {i} differs"
        );
    }
    for (id, expected) in reference.id_to_bytes.iter().enumerate() {
        assert_eq!(
            prod.vocab().get_bytes(id as u32),
            Some(expected.as_slice()),
            "{label}: vocabulary bytes differ at id {id}"
        );
    }
    for word in word_counts.keys() {
        let prod_ids = prod.encode_bytes(word);
        assert_eq!(
            prod.decode_bytes(&prod_ids).unwrap(),
            *word,
            "{label}: production byte round-trip failed"
        );
        let ref_ids = reference_encode(word, &reference);
        assert_eq!(
            reference_decode(&ref_ids, &reference),
            *word,
            "{label}: reference round-trip failed"
        );
    }
    let _ = std::fs::remove_file(path);
}

#[test]
fn differential_overlapping_occurrences_and_repeated_pairs() {
    // Heavy overlap: (a,a) appears twice in "aaa", three times in "aaaa",
    // and overlapping (a,b)/(b,a) pairs share nodes. The production trainer
    // must consume each occurrence exactly once per merge step.
    let map = counts(&[
        (b"aaaa", 20),
        (b"aaa", 11),
        (b"aa", 7),
        (b"aab", 9),
        (b"aabb", 6),
        (b"abab", 13),
        (b"ab", 17),
        (b"ba", 17),
        (b"abc", 12),
        (b"cab", 8),
    ]);
    differential_check("overlap", &map, 40, 2);
}

#[test]
fn differential_equal_frequency_tie_break() {
    // Four disjoint equal-frequency pairs; every step has a frequency tie.
    // Production and reference must both pick the lexicographically smallest
    // token-id pair at each tied step.
    let map = counts(&[
        (b"ab", 10),
        (b"ac", 10),
        (b"ad", 10),
        (b"ae", 10),
        (b"ba", 10),
        (b"ca", 10),
        (b"da", 10),
        (b"ea", 10),
    ]);
    differential_check("tie", &map, 6, 1);
}

#[test]
fn differential_repeated_token_sequences() {
    let map = counts(&[
        (b"abcabcabc", 50),
        (b"abcabc", 30),
        (b"abc", 20),
        (b"bcabca", 15),
        (b"cab", 10),
        (b"thequickbrownfox", 8),
        (b"thequick", 8),
        (b"quickbrown", 8),
        (b"brownfox", 8),
    ]);
    differential_check("repeat", &map, 50, 2);
}

#[test]
fn differential_unicode_bytes() {
    let map = counts(&[
        ("Türkçe".as_bytes(), 40),
        ("İstanbul".as_bytes(), 20),
        ("Şemsi".as_bytes(), 15),
        ("العربية".as_bytes(), 30),
        ("हिन्दी".as_bytes(), 25),
        ("中文测试".as_bytes(), 20),
        ("日本語テスト".as_bytes(), 20),
        ("한국어".as_bytes(), 18),
        ("Ελληνικά".as_bytes(), 12),
        ("🚀👨‍👩‍👧‍👦".as_bytes(), 10),
        ("a\u{0301}e\u{0302}".as_bytes(), 8),
    ]);
    differential_check("unicode", &map, 40, 2);
}

#[test]
fn differential_very_short_and_empty_corpus() {
    // Empty corpus: no merges, no words.
    let empty: HashMap<Vec<u8>, usize> = HashMap::new();
    differential_check("empty", &empty, 10, 1);

    // No adjacent pair at all.
    let one = counts(&[(b"a", 30)]);
    differential_check("one_byte", &one, 10, 1);

    // Exactly one possible merge.
    let single = counts(&[(b"ab", 1)]);
    differential_check("single_pair", &single, 10, 1);
}

#[test]
fn differential_min_frequency_boundaries() {
    // Counts 5, 4, 3, 2, 1 with min_frequency 3: only the first three pairs are eligible.
    let map = counts(&[(b"ab", 5), (b"cd", 4), (b"ef", 3), (b"gh", 2), (b"ij", 1)]);
    differential_check("minfreq3", &map, 10, 3);

    // No pair reaches the cutoff.
    differential_check("minfreq6", &map, 10, 6);

    // min_frequency 1 includes every pair, including count-1 pairs.
    differential_check("minfreq1", &map, 10, 1);
}

#[test]
fn differential_long_corpus() {
    let mut map = HashMap::new();
    for i in 0..220usize {
        let word = format!("tok{:03}ab{:03}cd{:02}ef", i, (i * 17) % 997, i % 31);
        let count = (i % 23) + 1;
        *map.entry(word.into_bytes()).or_insert(0) += count;
    }
    for i in 0..40usize {
        let word = format!("shared{}abcabc{}", i % 7, i % 11);
        *map.entry(word.into_bytes()).or_insert(0) += 5;
    }
    differential_check("long", &map, 120, 2);
}

#[test]
fn production_tie_break_is_stable_across_repeated_runs() {
    let map = counts(&[
        (b"ab", 10),
        (b"ac", 10),
        (b"ad", 10),
        (b"ba", 10),
        (b"ca", 10),
        (b"da", 10),
    ]);
    let path = temp_corpus_for("stable", &map);
    let mut manifest = CorpusManifest::new(42);
    manifest.add_entry(CorpusManifestEntry {
        path: path.clone(),
        weight: 1.0,
        language: None,
        domain: None,
        text_field: None,
    });
    let config = TrainingConfig {
        target_vocab_size: 256 + 8 + 2,
        min_frequency: 1,
        pretokenizer: PretokenizationMode::RawByte,
        normalization: Normalization::None,
        special_tokens: vec!["<|bos|>".to_string(), "<|eos|>".to_string()],
        seed: 42,
    };
    let (a, _) = train_from_manifest(&manifest, &config).unwrap();
    let (b, _) = train_from_manifest(&manifest, &config).unwrap();
    assert_eq!(a.merges().rules(), b.merges().rules());
    assert_eq!(a.vocab().get_bytes(300), b.vocab().get_bytes(300));
    let _ = std::fs::remove_file(path);
}
