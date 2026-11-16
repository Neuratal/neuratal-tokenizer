# NTokenizer v0.2 — Training Report (FINAL, measured)

All measurements are from the actual v0.2 run on this machine (2026-11-14 / 15).

## Configuration

| Setting | Value |
|---|---|
| Model | output/v0.2/ntokenizer-0.2-256k.ntok |
| Vocab | 262,144 |
| min_frequency | 2 |
| seed | 42 |
| normalization | None |
| pretokenizer | LeadingWhitespaceUnicode (lws) — ablation winner, see `reports/v0.2/ABLATION.md` |
| corpus manifest | data/v0.2/manifest/manifest_corpus.json (8,675 files, 1,018,783,149 B) |
| manifest SHA-256 | **205b25175357acbaac7769d9c70815194b4e38f6914a7ac34efa64273021d251** (embedded in the model metadata) |
| training command | `ntokenizer train --manifest data/v0.2/manifest/manifest_corpus.json --output output/v0.2/ntokenizer-0.2-256k.tmp.ntok --vocab-size 262144 --min-freq 2 --pretokenizer lws --seed 42` |

## Measured run

| Metric | Value |
|---|---|
| Records read | **13,885,542** |
| Bytes processed | **1,018,783,149 B** |
| Unique pre-tokens | **10,820,251** |
| Initial pair count | **n/a (initial pair count not printed by this CLI build)** |
| Merges learned | **261,881 (max allowed at vocab 262,144)** |
| Skipped duplicate merges | **0** |
| Final vocab size | **262,144** |
| Malformed records | **0** |
| Training wall time | **807,581 ms (~13.5 min)** |
| Peak memory (RSS) | **~9.7 GB (sampled /proc/<pid> VmRSS; swap untouched with MALLOC_ARENA_MAX=2)** |
| Model SHA-256 | **1fc3ee8568b7a08a124a5e27ee7a8078d5bdfae495a253b3f97ba0ad29ab54ac** |
| Format | NTOK v3 (expected; CRC + payload-length verified on load) |

## Determinism

Same corpus, seed, configuration ⇒ byte-identical merge history and model.
Verified: two runs with `SOURCE_DATE_EPOCH=1763107200` produced identical
SHA-256 (`8d382366961d35fb656d6d0bc3843176296e015417a429e6d76682040df907a8`).
Without the env var, only wall-clock metadata fields differ. BPE reference
differential tests (`tests/bpe_reference_tests.rs`, 11 tests) pass; removed-then-
re-add of interleaved merges reproduces the reference implementation exactly.

## Training corpus composition (see `data/v0.2/reports/corpus_distribution.json`)

- english 260 MB (25.5%), programming 140 MB (13.8%), turkish 95 MB (9.3%),
  CJK 267 MB (26.3%), STEM 90 MB (8.9%), hindi 53 MB, spanish 50 MB, arabic 38 MB
- Leakage against benchmark: **0** (file-exact/normalized/containment; see
  `leakage_audit.json`); train honored the audit quarantine (108 files moved).

## Model-family exports

All from the SAME merge history (first-N exports), each independently loaded,
validated, round-tripped and benchmarked — see `BENCHMARK_REPORT.md`:

| Export | vocab | size (bytes) | SHA-256 |
|---|---|---|---:|
| output/v0.2/ntokenizer-0.2-256k-8192.ntok | 8,192 | 547,598 | 8fa5743d8368bfbf02fbf949a331f5e9be63af37f6257c1d37ab766abfb6274c |
| output/v0.2/ntokenizer-0.2-256k-16384.ntok | 16,384 | 1,167,705 | 57a44b2dffcf68ea507459bfdbed41840788927bd8864a109ae31afbf7963f75 |
| output/v0.2/ntokenizer-0.2-256k-32768.ntok | 32,768 | 2,471,045 | 3bf7bceaaea9f625405a1933c844751b46bdd91ea42aef8b02e28f79a3d0c96c |
| output/v0.2/ntokenizer-0.2-256k-65536.ntok | 65,536 | 5,184,395 | 47374f64ec9cffe9f92b19eff91da7a84a1f4f7d392766ebe4a1d43b0e7ab7af |
| output/v0.2/ntokenizer-0.2-256k-131072.ntok | 131,072 | 10,859,866 | f29f2314b35ba7762c214c16964efb91aa9ec223444bafaec66b6107bc1207d2 |
| output/v0.2/ntokenizer-0.2-256k-196608.ntok | 196,608 | 16,729,305 | 99627cbcd1a5ceea5181ad39bff805afe6af2c1de09876d90ffcf3b57f1f8098 |
| output/v0.2/ntokenizer-0.2-256k-262144.ntok | 262,144 | 22,712,478 | 516704c91dcbc557bd5a1bb58fccfc1dbf7d4a8b8b42a497f85cda0c0bab4e48 |
| output/v0.2/ntokenizer-0.2-256k.ntok (main) | 262,144 | 22,712,471 | 1fc3ee8568b7a08a124a5e27ee7a8078d5bdfae495a253b3f97ba0ad29ab54ac |

Independent validation of every export (`ntokenizer validate --model <file>`,
plus CRC/vocab/merge/special-token/roundtrip checks) — results in
`reports/v0.2/release_validation.log`.

## Metadata

`output/v0.2/MODEL_METADATA.json` (+ embedded `metadata` object in the v3
payload). Contents: model name/version, format version, crate version,
training time bounds, corpus manifest SHA-256, corpus bytes/records, pretokenizer,
seed, min frequency, CLI string, licenses reference, git commit (unborn branch
→ empty with note). Bound to the model with SHA-256.
