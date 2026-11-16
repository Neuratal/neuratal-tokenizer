# NTokenizer v0.3 License Audit

Date: 2026-11-15
Status: audited. This file records the licensing of the **code**, the **training
corpus**, and the **distributed model artifacts** for v0.3. It repeats no
third-party copyright text verbatim; it references the source documents listed in
§5. Nothing here is legal advice; distribution decisions remain the redistributor’s.

## 1. Code (this repository)

| Component | License | Notes |
|---|---|---|
| `neuratal-tokenizer` crate source (`src/`, `examples/`, `tests/`, `fuzz/`) | **MIT OR Apache-2.0** at the recipient’s option (see `LICENSE`, headers added in v0.2) | Apache-2.0 is the preferred license per project policy |
| `python/neuratal_tokenizer` (ctypes bindings) | MIT OR Apache-2.0 | same |
| Build system (`Cargo.toml`) | MIT OR Apache-2.0 | same |
| Documentation (`docs/`, `README.md`, `reports/`) | MIT OR Apache-2.0, with factual/data provenance as listed in §5 | documentation text is original |
| `THIRD_PARTY_NOTICES.md` | n/a | lists transitive dependency licenses |

## 2. Third-party Rust dependencies (transitive)

`Cargo.lock` pins: serde / serde_json / serde derive (MIT OR Apache-2.0), rayon
(MIT OR Apache-2.0), unicode-general-category and unicode-script (MIT), crc32fast
is not used (CRC32 is implemented in-repo, `src/serialization/format.rs`),
aes-gcm was not used. `unicode-general-category`/`unicode-script` carry the
Unicode Data Files (UCD) license terms of https://www.unicode.org/license.html
(free use, attribution in `THIRD_PARTY_NOTICES.md` recommended). No dependency is
strong-copyleft; no dependency relicenses the project.

## 3. Training corpus (what is actually inside the v0.3 model)

The v0.3 corpus is `data/v0.3/manifest/manifest_corpus.json`
(SHA-256 `2b5a5c523d28b76aa11c371c312814be400cce82d95a0ec37c989b0a7a2cc186`),
drawn **exclusively** from the v0.2 leakage-audited `data/v0.2/clean/train` tree.
No new text was downloaded for v0.3. Aggregate composition of the final manifest:

| Source | Bytes | Files | Bucket | License | Share |
|---|---:|---:|---|---|---:|
| arwikisource | 293.3 MB | 130 | arabic | CC BY-SA 4.0 + GFDL (mixed CC0 pages; treated as CC BY-SA) | 14.9% |
| trwiki | 230.6 MB | 110 | turkish | CC BY-SA 3.0 Unported + GFDL | 11.7% |
| hiwiki | 229.9 MB | 109 | hindi | CC BY-SA 4.0 + GFDL | 11.7% |
| kowikisource | 177.9 MB | 84 | korean | CC BY-SA 4.0 + GFDL | 9.1% |
| eswikisource | 146.8 MB | 69 | spanish | CC BY-SA 4.0 + GFDL | 7.5% |
| arXiv (187 per-article-verified CC BY papers) | 125.8 MB | 2,177 | stem | **CC BY 4.0** | 6.4% |
| gutenberg_cjk (435 zh + 21 ja books) | 122.0 MB | 335 | chinese/japanese | Public domain (US); PG terms per book | 6.2% |
| jawikisource | 113.6 MB | 52 | japanese | CC BY-SA 4.0 + GFDL | 5.8% |
| simplewiki | 104.3 MB | 50 | english | CC BY-SA 4.0 + GFDL | 5.3% |
| cpython (code + docs) | 63.0 MB | 2,876 | programming | PSF License 2.0 | 3.2% |
| gutenberg_top100 / gutenberg_en | 47.1 MB | 61 | english | Public domain (US); PG terms per book | 2.4% |
| trwikisource | 47.0 MB | 22 | turkish | CC BY-SA 4.0 + GFDL | 2.4% |
| enwikibooks | 46.6 MB | 23 | english | CC BY-SA 4.0 + GFDL | 2.4% |
| gutenberg_science2 | 34.5 MB | 112 | stem | Public domain (US); PG terms per book | 1.8% |
| typescript_src | 31.7 MB | 686 | programming | Apache-2.0 | 1.6% |
| go_src | 30.4 MB | 2,117 | programming | BSD-3-Clause | 1.5% |
| gutenberg_science | 28.7 MB | 72 | stem | Public domain (US); PG terms per book | 1.5% |
| enwikinews | 20.9 MB | 10 | english | CC BY 2.5 (pre-2024-12-16) / CC BY 4.0 — not ShareAlike | 1.1% |
| rust_src | 15.6 MB | 1,419 | programming | MIT OR Apache-2.0 | 0.8% |
| zhwiktionary | 14.7 MB | 8 | chinese | CC BY-SA 4.0 + GFDL | 0.7% |
| jawiktionary | 14.3 MB | 7 | japanese | CC BY-SA 4.0 + GFDL | 0.7% |
| trwiktionary | 9.3 MB | 5 | turkish | CC BY-SA 4.0 + GFDL | 0.5% |
| kowiktionary | 9.1 MB | 5 | korean | CC BY-SA 4.0 + GFDL | 0.5% |
| tatoeba (16 langs) | 6.4 MB | 16 | arabic…other_languages | CC BY 2.0 FR | 0.3% |
| **total** | **1,963.48 MB** | **10,555** | | | 100% |

Exact per-entry `source`, `bucket`, `domain`, `bytes`, `sha256` and `license` are
recorded in the manifest itself and validated by `experiments/v0.3/build_v03_manifest.py`.

## 4. Distribution obligations for the v0.3 model artifact

The v0.3 `.ntok` model (and the Hugging Face export `output/v0.3/hf/`) is a
**machine-learned derivative** of this corpus. The applicable obligations:

1. **CC BY-SA sources** (most wiki dumps: simplewiki, trwiki, trwikisource,
   trwiktionary, hiwiki, eswikisource, arwikisource, enwikibooks, jawikisource,
   kowikisource, jawiktionary, kowiktionary, zhwiktionary) require attribution and
   ShareAlike of adaptations under the same license. The model must therefore be
   distributed under terms compatible with CC BY-SA 4.0/3.0 + GFDL, with the
   attribution string in `data/v0.2/LICENSES.md`, e.g. `<project> contributors,
   <license>, <URL>`. Declaring the model "Apache-2.0-only" without those notices
   would misrepresent its obligations.
2. **CC BY sources** (arXiv per-article CC BY 4.0; enwikinews pre-2024 text under
   CC BY 2.5 / later under CC BY 4.0; Tatoeba CC BY 2.0 FR) require attribution
   only, no ShareAlike.
3. **Permissive/public-domain sources** (CPython PSF 2.0, Rust MIT/Apache-2.0, Go
   BSD-3-Clause, TypeScript Apache-2.0, Project Gutenberg public-domain texts)
   require only retention of notices/headers, which the cleaner preserves in code
   and does not strip from books except the standing PG boilerplate markers.
4. **arXiv quarantine is NOT used anywhere.** The 187 CC BY papers in the corpus are
   the only arXiv content admitted; the 293 non-CC-BY papers were quarantined in
   v0.2 (`data/v0.2/LICENSES.md`, `ARXIV_LICENSE_AUDIT.json`) and are not in any
   v0.3 manifest or model.
5. **No benchmark contamination:** benchmark files are exclusively in
   `data/v0.2/clean/benchmark`; the corpus is exclusively derived from
   `clean/train`; leakage audit results are referenced in §6.
6. **No relicensing of third-party data is claimed.** The project's MIT/Apache-2.0
   code license applies to the implementation. The trained token vocabularies and
   merge tables derived from CC BY-SA text remain subject to §(1).

Practical consequence: redistribute `ntokenizer-0.3-256k.ntok` and
`output/v0.3/hf/` **with** `LICENSE`, `THIRD_PARTY_NOTICES.md`,
`data/v0.2/LICENSES.md`, and this audit, under CC BY-SA-compatible terms. Do not
claim an Apache-2.0-only model.

## 5. Upstream provenance documents (not duplicated here)

- `data/v0.2/LICENSES.md` — source list, dumps, SHA-256, redistribution notes (v0.3 inherits this unchanged)
- `data/v0.2/manifest/raw_provenance.json` — 236,239 raw files, 7.60 GB, per-file SHA-256/URL/license/retrieval date
- `data/v0.2/CORPUS.md`, `data/v0.2/LICENSES.md`, `data/main/CORPUS.md`, `data/main/LICENSES.md`, `data/pilot/LICENSES.md`
- `reports/v0.2/FINAL_REPORT.md` (§3 binary filtering, §10 CJK/STEM coverage)
- `reports/v0.3/corpus_distribution.json` — final v0.3 manifest statistics
- `LICENSE`, `THIRD_PARTY_NOTICES.md`, `CONTRIBUTING.md` — project licensing

## 6. Benchmark legality & leakage

The benchmark is a fixed, held-out corpus of the same provenance as the training
data. No benchmark file, byte, or near-duplicate was admitted to training
(`reports/v0.2/leakage_audit.json` — passed with max containment 0.594 vs ≥0.6
threshold; 631 files quarantined in v0.2; zero v0.3 corpus entries touch
`clean/benchmark`). Evaluating on it is fair use of the redistribution terms of the
sources for measurement purposes inside this project; public release of the
benchmark would itself need the same attribution handling. The report does not
publish benchmark text.

## 7. Known gaps / not done

- Gutenberg per-book headers were not individually re-verified for v0.3; the
  eligibility decision relies on the v0.2 audit of `gutenberg_cjk`,
  `gutenberg_top100`, `gutenberg_science`, `gutenberg_science2`, which checked all
  456 CJK books and the science collections at import time.
- Wikimedia dump XML is not line-versioned per article; the audit treats each dump
  as of 2026-11-14 retrieval date. Pages-deleted-since-then is out of scope.
- `tatoeba` sentences are contributed by thousands of volunteers under CC BY 2.0 FR;
  attribution is given to Tatoeba and contributors collectively, per the CC BY 2.0
  attribution guidelines.
- The v0.3 model was not retrained against a freshly downloaded corpus, so stale
  page removals after 2026-11-14 are not reflected — no legal issue, but an
  accuracy-relevant limitation.
