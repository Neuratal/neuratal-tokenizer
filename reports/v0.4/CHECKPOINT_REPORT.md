# NTokenizer v0.4 Checkpoint Report

Date: 2026-11-15
Status: v0.4-fail archived and verified; v0.3 checkpoint secured.

## 1. v0.4 (previous attempt) archived

The previous v0.4 candidate and all of its artifacts were moved to `*-fail/`
directories, preserving the original relative layout:

| Original | Archived |
|---|---|
| `output/v0.4/` | `output/v0.4-fail/` |
| `reports/v0.4/` | `reports/v0.4-fail/` |
| `experiments/v0.4/` | `experiments/v0.4-fail/` |

The archived model hash was verified **unchanged** by the move:

```
fbb5d2d386071f94993130ee2a196e02990c240307d56996487ee6878590874e  output/v0.4-fail/ntokenizer-0.4-256k.ntok
```

(`output/v0.4/ntokenizer-0.4-256k.ntok` SHA-256 before the move == after the
move — verified.)

`data/v0.4` (Benchmark v1 holdout/validation manifests and the audit farm)
remains in place: its manifests reference absolute paths under `data/v0.4/` and
its files are used by the new evaluation. The archived `reports/v0.4-fail/SHA256SUMS`
also references paths that moved with the directories (e.g.
`experiments/v0.4/results/...` → `experiments/v0.4-fail/results/...`); the archive
is a historical record — re-verify with
`sed 's#experiments/v0.4/#experiments/v0.4-fail/#' reports/v0.4-fail/SHA256SUMS | sha256sum -c`
rather than by editing archived files.

## 2. v0.3 checkpoint

| Item | Value |
|---|---|
| Model | `output/v0.3/ntokenizer-0.3-256k.ntok` |
| SHA-256 | `1f0d4ad7c7fb609001b2af4d9741ec5b8a5d7686fa3b92eb8eefd057fea936d9` |
| Format | NTOK v3, MathSymbolAware, seed 42, mwf=2, corpus 1.963 GB |
| Code reference after this session's additive v0.4 work | builds clean (`cargo build --release`), v0.3 benchmark reproduces **exactly**: 37,397,108 tokens, 5.5543 / 5.1057 B/T (see `BASELINE_REPORT.md`) |

The v0.3 model is read-only and untouched. A full source snapshot of the working
tree was taken before any v0.4 work this session:

```
/tmp/opencode/v04_source_backup_2026-11-15.tar.gz
sha256 6a45fc5b936a4037c934beaf384521b9eb2530cba0782f4e3e3c03d5a9a3ef50
```

(170 KB; contains `src/`, `Cargo.toml`, `Cargo.lock`, `examples/`, `tests/`,
`docs/`, `python/`, `fuzz/`, `README.md`, `CHANGELOG.md`.)

## 3. Why v0.4 (previous) was archived as failed

- overall B/T 5.5511 < v0.3's 5.5543 (−0.06%), equal-category 5.1032 vs 5.1057
- japanese −0.64% vs v0.3 (cjk8), japanese −2.6% (cjk4)
- encode speed unchanged (10.18 MB/s), inference RSS slightly up (3.75 vs 3.70 GiB)
- HF export: token-ID parity 46/523
- CJK-exempt arm explodes the pair index at production scale (8.3 GB at 3.5% of
  merges) — infeasible

## 4. New v0.4 policy

- Starts strictly from the v0.3 checkpoint (model + manifest + corpus).
- Only measured-win arms are considered for release; v0.3 stays default unless a
  candidate beats it on the acceptance table (`FINAL_REPORT.md`).
- No v0.4-fail experiments are re-run at full scale without a pilot-scale win first.
