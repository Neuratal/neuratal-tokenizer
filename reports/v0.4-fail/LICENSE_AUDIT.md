# NTokenizer v0.4 — License Audit

Date: 2026-11-15
Status: audited. Code, dependencies, corpus, learned artifacts, and distribution are
analyzed **separately**; this is deliberately not a single "the model is licensed
X" statement. Nothing here is legal advice.

## 1. Code (this repository)

| Component | License | Notes |
|---|---|---|
| `neuratal-tokenizer` crate (`src/`, `examples/`, `tests/`, `fuzz/`) | **MIT OR Apache-2.0** at the recipient's option | Apache-2.0 is the preferred license per project policy; dual license matches `LICENSE` and v0.2/v0.3 |
| `python/neuratal_tokenizer` (ctypes bindings) | MIT OR Apache-2.0 | same |
| `python/pyproject.toml` | MIT OR Apache-2.0 | same |
| Documentation (`docs/`, `README.md`, `CHANGELOG.md`, `reports/`) | MIT OR Apache-2.0, factual provenance as per §4–§6 | text is original to this project |
| `THIRD_PARTY_NOTICES.md` | lists transitive dependency licenses | serde/rayo (MIT/Apache-2.0), unicode-* (MIT/Unicode license), clap (MIT/Apache-2.0) |

New v0.3/v0.4 code (pretokenizer modes `MathSymbolAwareCjk2/4/8`, the
`--min-word-freq-cjk` script-aware trainer floor, the benchmark harness, the HF
export tool and sidecar generator, the audit/build scripts) is original work of
this project and is released under the same MIT OR Apache-2.0.

## 2. Rust dependencies (transitive)

No new dependencies were added in v0.4. The tree remains: serde/serde_json
(MIT/Apache-2.0), rayon (MIT/Apache-2.0), clap (MIT/Apache-2.0),
unicode-general-category and unicode-script (MIT), unicode-normalization (MIT).
No dependency is copyleft; no dependency relicenses the project. The two Unicode
data crates embed the Unicode Character Database (UCD), whose use is governed by
https://www.unicode.org/license.html (attribution retained in
`THIRD_PARTY_NOTICES.md`).

## 3. Training corpus (exactly what the v0.4 release candidate trains on)

**The v0.4 release candidate is trained on the same corpus as v0.3**
(`data/v0.3/manifest/manifest_corpus.json`, SHA-256 `2b5a5c523d28b76aa11c371c312814be400cce82d95a0ec37c989b0a7a2cc186`),
a leakage-audited 1.96 GB sub-selection of `data/v0.2/clean/train`
(5.14 GB, 12,590 files), itself derived from the v0.2 clean pipeline. The v0.4
corpus hash therefore equals the v0.3 corpus hash by construction.

Per-source composition (identical to the v0.3 audit):

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
| cpython (code+docs) | 63.0 MB | 2,876 | programming | PSF License 2.0 | 3.2% |
| gutenberg_top100 / gutenberg_en | 47.1 MB | 61 | english | Public domain (US); PG terms | 2.4% |
| trwikisource | 47.0 MB | 22 | turkish | CC BY-SA 4.0 + GFDL | 2.4% |
| enwikibooks | 46.6 MB | 23 | english | CC BY-SA 4.0 + GFDL | 2.4% |
| gutenberg_science2 | 34.5 MB | 112 | stem | Public domain (US); PG terms | 1.8% |
| typescript_src | 31.7 MB | 686 | programming | Apache-2.0 | 1.6% |
| go_src | 30.4 MB | 2,117 | programming | BSD-3-Clause | 1.5% |
| gutenberg_science | 28.7 MB | 72 | stem | Public domain (US); PG terms | 1.5% |
| enwikinews | 20.9 MB | 10 | english | CC BY 2.5 (pre-2024-12-16) / CC BY 4.0 — **not** ShareAlike | 1.1% |
| rust_src | 15.6 MB | 1,419 | programming | MIT OR Apache-2.0 | 0.8% |
| zhwiktionary | 14.7 MB | 8 | chinese | CC BY-SA 4.0 + GFDL | 0.7% |
| jawiktionary | 14.3 MB | 7 | japanese | CC BY-SA 4.0 + GFDL | 0.7% |
| trwiktionary | 9.3 MB | 5 | turkish | CC BY-SA 4.0 + GFDL | 0.5% |
| kowiktionary | 9.1 MB | 5 | korean | CC BY-SA 4.0 + GFDL | 0.5% |
| tatoeba (16 langs) | 6.4 MB | 16 | mixed | CC BY 2.0 FR | 0.3% |
| **total** | **1,963.48 MB** | **10,555** | | | 100% |

A separate **v0.4-A validation artifact** model (`experiments/v0.4/models/final-A_math_mwf2_256k.ntok`)
was trained on a 1.93 GB sub-corpus that additionally excludes the Benchmark v1
carved holdout/validation files (1,487 files, 35.1 MB; tatoeba `other_languages`,
Chinese Gutenberg, code, selected arXiv papers). That model is not the release
candidate and is used only to probe whether v0.4-style training can be made
fully holdout-clean; its license picture is the same sources minus those files.

## 4. The CC BY-SA question (stated precisely, not resolved by assertion)

The largest sources (Wikimedia) are CC BY-SA (3.0/4.0) + GFDL. Facts:

- The cleaned texts were transformed in mechanical but nontrivial ways (extraction,
  cleaning, dedup, re-chunking, byte-level re-encoding in the tokenizer).
- A trained vocabulary + merge table is a **statistical summary** of the corpus,
  not a "copy" of any article; no sequence in the model is reproducible in bulk
  from the data without the training set.
- Whether a tokenizer model is a "derivative work" of CC BY-SA texts is a legal
  interpretation our project is **not competent to settle**, and published
  guidance is not uniform.
- What the project can and does state factually:
  1. The corpus text is CC BY-SA / CC BY / public-domain / permissively licensed.
  2. Attribution URLs and retrieval dates are recorded (`data/v0.2/LICENSES.md`,
     `data/v0.2/manifest/raw_provenance.json`).
  3. The model is distributed under the project's MIT/Apache-2.0 **code** license
     plus the corpus-source obligations documented here; it is **not** labeled
     Apache-2.0-only without that notice.
  4. If a downstream user cannot comply with CC BY-SA (attribution + ShareAlike),
     they must not redistribute the model; training from the published manifests
     is likewise possible.
- Recommended distribution posture: distribute the model with
  `THIRD_PARTY_NOTICES.md`, `data/v0.2/LICENSES.md` and this audit, and treat
  the vocabulary/merge table as possibly-ShareAlike for the BY-SA-covered
  fraction. Do **not** claim the artifact is CC0 or Apache-2.0-only.
- The 187 arXiv papers are CC BY 4.0 (attribution required, no ShareAlike).
  Gutenberg texts are US-public-domain (some PG-EU/per-book terms; all headers
  retain the book's own notice). Tatoeba is CC BY 2.0 FR (attribution).
- No relicensing of any third-party content is claimed anywhere in this project.

## 5. Benchmark corpora

- `data/v0.2/clean/benchmark` (544 files) — derived from the same cleaned sources;
  same license families as the training corpus; it is **derived works of the
  sources**, and the v0.2 redistribution analysis applies. It is not redistributed
  with this project; publishing it would need its own attribution pass.
- `data/v0.4/benchmark/{holdout,validation,unicode_stress}` — same provenance
  (subset of the same clean tree) plus synthetic license-free fixtures
  (`experiments/v0.4/build_unicode_stress.py`, CC0-1.0). Same analysis applies;
  not redistributed in this audit's scope.

## 6. Learned artifacts (model, vocab, merge table, HF export)

- `output/v0.3/ntokenizer-0.3-256k.ntok` — the v0.3 baseline (source of truth for
  this audit's baseline numbers). Distribution posture per §4.
- `output/v0.4/...` (release candidate, when finalized) — same posture.
- `output/v0.3/hf/tokenizer.json` (and the v0.4 equivalent, when produced) —
  contains the same learned vocabulary and merge list serialized for HF. Its
  license obligations are the same as the `.ntok` model (it is the same artifact
  in different packaging); it additionally embeds no project code, so the code
  license question is separable from the data-artifact question.
- No OOV weights, no embeddings, no pretrained activation vectors from any
  external model are included anywhere. `tiktoken` encodings, Qwen weights, and
  any other third-party tokenizer files are **not** bundled or copied; they are
  referenced only as measurement baselines.

## 7. External tools used for research/evaluation (not part of the product)

| Tool | License | Use in v0.4 |
|---|---|---|
| tiktoken (OpenAI) | MIT | o200k/cl100k/GPT-2 baselines (measurement only) |
| swiss-ai/parity-aware-bpe | MIT | parity-aware merge-selection pilot (subprocess; code not copied into NTokenizer) |
| SentencePiece (Google) | Apache-2.0 | Unigram reference pilot (subprocess only) |
| tokenizers / transformers (HF) | Apache-2.0 | HF export validation |
| PyTorch | BSD-3-Clause | 50M-model pilot (if run) |

Note: SentencePiece is **Apache-2.0** (per `pip show sentencepiece` License-Expression;
Google's SentencePiece is Apache-2.0). It is never linked into NTokenizer; used only
via CLI in an isolated venv.

## 8. Verification performed

- `python3 experiments/v0.3/build_v03_manifest.py` output checked; no entry path
  points into `clean/benchmark`.
- `data/v0.4/_audit_train_farm` symlink audit passed (`reports/v0.4/leakage_audit.json`).
- `reports/v0.3/corpus_distribution.json` cross-checked against manifest sums.
- All source licenses above are from `data/v0.2/LICENSES.md`,
  `data/main/LICENSES.md`, `data/pilot/LICENSES.md`, and `raw_provenance.json`
  (SHA-256 of every raw file, retrieval dates 2026-11-14).

## 9. Not done / honest gaps

- Gutenberg per-book headers were not individually re-verified for v0.4; the
  eligibility decision rests on the v0.2-time audit of `gutenberg_*` (all 456 CJK
  books were checked then). Any book whose header restricts redistribution would
  matter only for redistribution of the raw bytes, not for model training claims —
  but it has not been re-litigated here.
- The CC BY-SA-to-tokenizer question has no definitive published ruling; §4
  states positions, not conclusions.
- The v0.4 model is not published; if it is published, the distribution posture
  in §4 must accompany it, and the project's NOTICE/attribution must be updated.
- `MANUAL`: I did not read every one of the 236,239 raw-file provenance entries;
  I relied on the v0.2 audit that did. That audit's own limitations (e.g., wiki
  dumps not line-versioned) apply here.
