# NTokenizer v0.1 — Verified Root Causes of Reported Issues

Verification date: 2026-11-14. All checks ran against the actual files in this repository.
The v0.1 baseline model was not modified:

```
sha256(output/ntokenizer-256k.ntok) = 8abe252f159ed58a1a0072eaac0f9861dab74b4e6bed5da9b69a218493662f00
```
(matches the published baseline hash exactly.)

## 1. 91 binary-contaminated files (~19.25 MB) — CONFIRMED (CR-1)

Method: walked `data/main/clean/{train,benchmark}`, counted NUL bytes and unexpected
control bytes (excluding `\t \n \r`), and attempted strict UTF-8 decoding.

- Files with ≥1 NUL byte: **91** (matches the reported count exactly)
- Files with control-character content (incl. NUL files): **95 files, 21,325,075 B (~20.3 MB)**, of which:
  - 4.5 MB `go_src .../runtime/race/...race_linux*` data blobs (e.g. `amd64v3_race_linux.txt` 0.45 ratio)
  - CPython `Lib/test/archivetestdata/*` (tar/zip test fixtures), `Lib/idlelib/Icons/*.png-as-.txt`
  - CPython `Lib/test/audiodata/pluck-*`, `Doc/library/tachyon-*` (PNG/screenshots embedded)
  - Rust `std_detect` test data `.txt-wasm` files (0.62–0.96 control ratio)
- **One benchmark file is contaminated**: `en_books_data_main_clean_wiki_en_books_en_books-0003.txt` (2,073,479 B, contains control bytes)
- Invalid-UTF-8 files: 0 (the v0.1 cleaner silently fell back to Latin-1, which cannot fail)

Root cause: `decode_file()` in `data/main/clean_new.py` tries `utf-8` then `latin-1`.
Latin-1 maps all 256 byte values, so the fallback "succeeds" for binary files and
**zero** files were rejected. No magic-byte, NUL, or control-char sniffing existed anywhere.

## 2. `line.trim()` destroying code indentation — CONFIRMED (CR-2)

`src/training/corpus.rs:92` (v0.1):
```rust
let trimmed = line.trim();
if trimmed.is_empty() { continue; }   // blank lines dropped entirely
```
Every training record (TXT) lost leading indentation and trailing newline handling was
implicit. A Python file `def outer():\n    if True:` trains on `def outer():` / `if True:`.
Additionally, **all newlines were removed from the training data**, so the model could
never learn `\n`/`\n    ` merges. The cleaner additionally did `raw_line.rstrip()` per line.

## 3. Near-duplicate Unicode false positives — CONFIRMED (CR-3)

`data/main/clean_new.py` lines 40–52:
```python
DEDUP_MIN_LEN = 100
NEARDUP_PREFIX = 200
def normalize(s):
    stripped = RE_PUNCT.sub(" ", s.lower())     # [^\w]+ -> space
    return RE_WS.sub(" ", stripped).strip()
```
Measured behavior:
- `"कि का कु"` → `"क क क"` — Devanagari matras (category Mn, matched by `[^\w]`) are deleted;
  distinct words collapse to the same base consonants and become false near-duplicates.
- `"İstanbul ... İ ve ı"` → `"i stanbul ... i ve ı"` — Turkish İ lowercases to `i̇` and the
  combining dot is then stripped; `İ` and `i` merge into the same key.
- Pure emoji line (48 bytes of repeated emoji >100 chars) → `""` — **empty key**; every such
  line hashes to the same near-dup key, so genuinely different emoji artwork lines were
  silently deleted after the first occurrence.
- ZWJ family-emoji line → `""` (same failure mode).
Root cause: a normalize-then-truncate (first 200 chars) design applied universally, with
lowercasing and punctuation stripping, applied to non-Latin scripts.

## 4. Benchmark overlap with train — CONFIRMED (CR-4)

Measured with exact line matching over `data/main/clean/{train,benchmark}`:
- **57,560 of 607,843 benchmark lines (9.5%) appear verbatim in train** — matches the v0.1 report exactly.
- Of those, ZERO are >100-char lines; the leakage is short common lines + license headers
  (Gutenberg 7,479 in `gutenberg_pg100.txt`; Wikipedia shards 2,217–7,280; typescript_src 2,085;
  `tr_wiki2` 2,045).
- **5 benchmark files are byte-identical to a train file** (cpython_code `__future__`/`__hello__`
  ×2 code + 2 docs, go_src `export_aix_test`≡`export_darwin_test`).
- 12 pilot-train files were relocated into `data/main/clean/benchmark/` with no documentation.
- v0.1 near-dup seeds were loaded from the pilot corpus, but the benchmark split happened
  *before* the main split, and pilot↔main overlap across the split boundary was never re-checked.

## 5. English tokenization efficiency vs GPT-2 — REPRODUCED AS REPORTED (CR-5)

v0.1 in-repo benchmark (methodology: synthetic samples, `ntokenizer benchmark`):
NTokenizer 256K **2.54 B/tok** vs GPT-2 3.92 vs o200k 4.16 (English).
Root cause hypothesis to fix: `Conservative Unicocode` whitespace-separates pre-tokens,
so `" the"`-style tokens cannot be learned; measured again in v0.2 ablation.

## 6. Hindi matra deletion — CONFIRMED (CR-6)

Covered in (3): Devanagari combining marks are removed by `[^\w]+` because `\w` does not
match category Mn. Same class of bug for Arabic harakat (Mn) and Thai vowels.

## 7. Turkish `İ` misnormalization — CONFIRMED (CR-7)

Covered in (3). Additionally `s.lower()` in Python is locale-aware only for `\w`-matching
and maps `İ (U+0130)` → `i\u0307`; the following `[^\w]+` strips the combining dot,
colliding with plain `i`.

## 8. Emoji and ZWJ sequences — CONFIRMED (CR-8)

- Cleaner: emoji lines normalize to `""` → false near-dup deletion (see 3).
- Pretokenizer: `ConservativeUnicode` splits between the emoji and the following punctuation
  but keeps ZWJ sequences attached (joiners attach to the preceding run) — that part was OK.
  `ScriptAndDigitAware` also keeps ZWJ sequences intact (test `test_emoji_zwj_attachment`).
  The residual issue was purely the corpus cleaner deleting emoji-only lines.

## 9. CJK held-out data insufficient — CONFIRMED (CR-9)

Measured: **5,521 CJK characters** across 19 benchmark files (v0.1) — matches the report.
With 925 MB of English/Turkish/etc. text, this is ~0.006% — nowhere near a usable CJK benchmark.

## 10. Model metadata missing — CONFIRMED (CR-10)

`src/serialization/format.rs` defines `ModelMetadata` but `save_ntok_file` serializes only the
`Tokenizer` struct; `ModelMetadata` is never written to disk. `created_at` is a hard-coded
constant `"2026-11-14T00:00:00Z"`.

## 11. Documentation stale — CONFIRMED (CR-11)

- `data/main/CORPUS.md` claims: benchmark = "92 documents ~10 MB"; actual `manifest_benchmark.json`
  has **122 entries / 55,755,941 B** (and corpus = 9,706 / 924,997,007 B).
- `CORPUS.md` claims zero files skipped by binary detection; actual: 91 NUL + 4 more control-byte files.
- `LICENSES.md` contains no SHA-256 for any source dump.
- README/benchmarks documents do not reflect the v0.1 verification findings.

## Verdict
All eleven reported issues were reproduced in the actual repository files. v0.2 addresses
each of them with new pipeline stages (see `CHANGELOG.md` / `CORPUS.md` v0.2 sections).
