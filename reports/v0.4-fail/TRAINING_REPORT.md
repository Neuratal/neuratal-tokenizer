# NTokenizer v0.4 Training Report

Date: 2026-11-15
Environment: Intel Core i7-12700H, 14 GiB RAM, Ubuntu Linux, rustc/cargo 1.99.0.
Commands run exactly as logged; logs in `experiments/v0.4/logs/`; all artifacts
hashed in `reports/v0.4/SHA256SUMS`.

## 1. v0.4 candidate (release model, `cjk8`)

```bash
MALLOC_ARENA_MAX=2 /usr/bin/time -v target/release/ntokenizer train \
  --manifest data/v0.3/manifest/manifest_corpus.json \
  --vocab-size 262144 --min-freq 2 --min-word-freq 2 \
  --pretokenizer cjk8 --seed 42 \
  --output experiments/v0.4/models/final-RELEASE_math_cjk8_256k.ntok
# re-saved with model_name="ntokenizer-0.4-256k" -> output/v0.4/ntokenizer-0.4-256k.ntok
```

| Field | Value |
|---|---|
| Model | `output/v0.4/ntokenizer-0.4-256k.ntok` |
| SHA-256 | `fbb5d2d386071f94993130ee2a196e02990c240307d56996487ee6878590874e` |
| Size | 23,387,324 B |
| Format | NTOK v3, embedded metadata (name `ntokenizer-0.4-256k`) |
| Pretokenizer | `MathSymbolAwareCjk8` (`CJK runs capped at 8 chars`) |
| Vocabulary | 262,144 (256 + 261,881 merges + 7 special tokens) |
| Manifest | `data/v0.3/manifest/manifest_corpus.json`, 10,555 entries, 1,963,480,352 B |
| Records / bytes | 20,041,192 / 1.963 GB |
| Unique pre-tokens | 20,659,974 |
| Pruned at mwf=2 | 14,894,470 (72.1%); 5,765,504 used for merge learning |
| Merges learned | 261,881 (all) |
| Wall time | 4 min 38 s (277.7 s) |
| User / system CPU | 467.85 s / 908.08 s (495% CPU) |
| Peak training RSS | 3,934,728 KiB (≈ 3.75 GiB) |

## 2. Control and rejected arms

| Arm | Manifest | Pre-tokens | Pruned (kept) | Wall | Peak RSS | Outcome |
|---|---|---:|---|---|---|---|
| v0.4 candidate cjk8 | full v0.3 corpus | 20,659,974 | 14,894,470 (5,765,504) | 4:38 | 3.75 GiB | **ship** |
| v0.4 cjk4 (rejected) | full v0.3 corpus | 21,611,164 | 15,450,343 (6,160,821) | 10:42 | 3.56 GiB | rejected: japanese −2.6%, encode −7.5% |
| v0.4 A control (mwf2) | carved corpus (1.928 GB, 9,068 files) | 19,471,664 | 13,923,419 (5,548,245) | 3:59 | 3.76 GiB | ablation artifact |

Validation of the A-control confirms the corpus-exclusion effect: other_languages
(carved tatoeba) collapsed −28.8% on the old benchmark, proving the 20–30% of
non-CJK regression in that arm was data-removal, not algorithmic.

## 3. Corpus provenance

- `data/v0.3/manifest/manifest_corpus.json` — the v0.3 production manifest,
  built deterministically from `data/v0.2/clean/train` (leakage-audited in v0.2);
  v0.4's candidate uses the **same** corpus and config as v0.3, changing only the
  CJK chunking. No new data was added; no benchmark bytes anywhere.
- `data/v0.4/manifest/manifest_corpus.json` — the carved corpus (benchmark v1
  exclusion), retained solely as an ablation artifact.

## 4. Training-data leakage

None — identical corpus and seed as v0.3 (verified disjoint from both the old 544
benchmark and the new Benchmark v1 holdout/validation splits; see
`reports/v0.4/BENCHMARK_SPEC.md` §5 for the leakage audit of v0.4).

## 5. Reported anomalies

- cjk4 arm was 2.3× slower to train than cjk8 (more unique pre-tokens → more
  merges per unique item). 10:42 wall for a 20–30%-of-corpus CJK slice alone
  would have dominated the budget; chunking at 8 is the right cap.
- The re-save through `rewrite_model_metadata` only rewrites `model_name`; the
  merge table and vocabulary are bit-identical to the trained file.
- `ntokenizer validate --roundtrip` passes all 13 probes and byte-exact roundtrip
  for the candidate (`TEST_REPORT.md`).
