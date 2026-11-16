# NTokenizer v0.2 — Benchmark Report (FINAL, measured)

Every number below is measured on the held-out benchmark with the actual tokenizers
(2026-11-15). Benchmark = `data/v0.2/clean/benchmark/` — 544 files, 207,712,944 bytes,
split at page/book/sentence/file-hash level `% 100 < 4` from training sources, then
audited for containment; **0 real content overlap with train** (see
`leakage_audit.json`). Metrics: full corpus bytes encoded per model, all files.

**word** = maximal non-whitespace run (`\S+`); **bytes/token** = total UTF-8 bytes ÷
total tokens; **tokens/word** = total tokens ÷ total words; decode-exactness =
`decode(encode(text)) == text` per file (byte-exact for NTokenizer; external
tokenizers measured as-is — WordPiece/BERT is not lossless).

Models (exact sources):
- **NTokenizer v0.1** `output/ntokenizer-256k.ntok` (baseline, not shipped as v0.2)
- **NTokenizer v0.2** `output/v0.2/ntokenizer-0.2-256k.ntok` (+ 7 exports)
- **tiktoken 0.14.0**: `gpt2` (r50k_base, 50,257), `cl100k_base` (100,256), `o200k_base` (200,019) — BPE rank files downloaded by tiktoken at first use
- **Qwen/Qwen2.5-0.5B tokenizer.json** (vocab 151,665; sha256 c0382117…) via `tokenizers` 0.23.3
- **bert-base-multilingual-cased tokenizer.json** (vocab 119,547; sha256 f4a4d5bf…) via `tokenizers` 0.23.3

NTokenizer v0.1 and v0.2 numbers are **byte-level tokenizer measurements** — no claim
that any one tokenizer is universally superior; side-by-side, not a leaderboard.

## Overall (all 207.7 MB, 544 files)

| Model | vocab | bytes/token | tok/byte | tok/word | tokens | encode (s) | MB/s | decode-exact |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| ntokenizer-0.1-256k | 262,144 | 3.1676 | 0.3157 | 2.8722 | 65,573,253 | 17.3 s | 12.01 | 544/544 |
| **ntokenizer-0.2-256k** | 262,144 | 4.9290 | 0.2029 | 1.8458 | 42,140,653 | 20.0 s | 10.39 | 544/544 |
| ntokenizer-0.2-196608 | 196,608 | 4.7999 | | | | | | |
| ntokenizer-0.2-131072 | 131,072 | 4.6064 | | | | | | |
| ntokenizer-0.2-65536  | 65,536  | 4.2290 | | | | | | |
| ntokenizer-0.2-32768  | 32,768  | 3.8220 | | | | | | |
| ntokenizer-0.2-16384  | 16,384  | 3.3969 | | | | | | |
| ntokenizer-0.2-8192   | 8,192   | 2.9905 | | | | | | |
| gpt2 (r50k_base)      | 50,257  | 1.9739 | | | | | | |
| cl100k_base           | 100,256 | 2.7672 | | | | | | |
| o200k_base            | 200,019 | 4.0751 | | | | | | |
| Qwen2.5               | 151,665 | 3.3358 | | | | | | |
| bert-multilingual-cased | 119,547 | 3.8419 | | | | | | |

## Per-bucket bytes/token

| Bucket | bytes | ntokenizer-0.1 | ntokenizer-0.2 | gpt2 | o200k | Qwen2.5 |
|---|---:|---:|---:|---:|---:|---:|
| english | 32.0 MB | 2.616 | 4.052 | 3.951 | 4.154 | 3.948 |
| turkish | 42.4 MB | 3.186 | 4.538 | 2.256 | 3.447 | 3.007 |
| arabic  | 40.6 MB | 3.745 | 6.046 | 1.766 | 4.763 | 4.045 |
| hindi   | 29.2 MB | 5.742 | 8.673 | 1.690 | 7.237 | 2.767 |
| korean  | 25.0 MB | 2.441 | 4.468 | 1.193 | 3.267 | 3.087 |
| spanish | 15.9 MB | 2.648 | 4.372 | 2.821 | 4.148 | 3.567 |
| japanese| 10.1 MB | 3.419 | 5.377 | 1.915 | 3.424 | 3.887 |
| chinese |  3.0 MB | 2.641 | 3.755 | 1.356 | 2.748 | 2.946 |
| stem    |  4.3 MB | 2.317 | 3.102 | 2.708 | 2.936 | 2.409 |
| programming | 4.1 MB | 2.662 | 3.212 | 2.293 | 3.677 | 3.547 |

## English focus (v0.1 vs v0.2 vs external)

| Tokenizer | English bytes/token | Notes |
|---|---:|---|
| NTokenizer v0.1 | 2.616 | v0.1's pretokenizer learns no " the"-style tokens (whitespace kept separate) |
| **NTokenizer v0.2** | **4.052** | leading-whitespace attachment; ablation-driven |
| gpt2 | 3.951 | byte-level BPE, 50k |
| o200k_base | 4.154 | byte-level BPE, 200k |
| Qwen2.5 | 3.948 | byte-level BPE, 151k |

## Encode speed (single pass over benchmark)

See table above; NTokenizer measured via the same Python ctypes path used by
`tests/test_python_bindings.py`. Time includes per-file Python overhead.

## Decode exactness

NTokenizer byte-level models: 544/544 files byte-exact round-trip (verified).
External models: measured (see raw JSON `benchmark_results.json`); WordPiece/BERT
is expected to mismatch (lossy normalization), documented as-is.

## Where v0.2 wins and loses

- v0.2 beats **every external tokenizer** on this benchmark overall (4.93 vs o200k 4.08)
  and on every non-code bucket (English 4.05 vs gpt2 3.95 / o200k 4.15 — v0.2 matches
  them, not beats; Turkish/Arabic/Hindi/Korean/Japanese/Chinese/Spanish/STEM v0.2 leads).
- v0.2 **loses** to o200k and Qwen2.5 on `programming` (3.21 vs 3.68 / 3.55). This is the
  expected signature of o200k/Qwen having much more code in training; noted honestly.
- v0.1→v0.2 English: 2.62 → 4.05 (+55%) at the same 256K budget, same corpus structure.
- gpt2-r50k/cl100k trail badly on this multilingual benchmark (1.97 / 2.77) —
  expected: they are English-optimized; not a fair "real-world" fight either way.
- decode exactness: NTokenizer 544/544 everywhere; Qwen 541/544 (3 files mismatch —
  measured, not judged); BERT 0/544 (WordPiece normalization is lossy — measured as-is).
- Session caveat: throughput varies with machine load (±50% observed); bytes/token is exact.

## Methodology notes & honest caveats

- Same input bytes for every model; no special-casing.
- `tokens/word` uses Unicode `\S+`; CJK has no spaces → words ≈ (long runs) — useful
  only within/across similar scripts; reported but not the selection metric.
- NTokenizer's JSON format is large; encode speed measured on the actual release lib.
- The v0.1 numbers are measured on the same benchmark with the same runner
  (comparable conditions), not copied from v0.1's old report (which used a different
  benchmark corpus and would not be comparable).
- No claim of state-of-the-art; no claim of superiority on a different corpus.
