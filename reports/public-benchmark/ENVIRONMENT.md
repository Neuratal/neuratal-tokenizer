# NTokenizer public benchmark — environment (2026-11-16)

## Machine

- Platform: Linux x86_64 (Ubuntu; kernel from `platform.release()`)
- CPU: 20 cores (nproc)
- RAM: ~14 GB
- Build: rustc 1.99.0 (release profile, `--locked`), Python 3.14.4 (system), Python 3.14 venv `.venv/` for external tokenizers (tiktoken 0.14.0, tokenizers 0.23.3)
- cargo registry: crates.io; NTokenizer built from this repository at the polished revision (see `PUBLICATION_REPORT.md` / git HEAD `b8866bf5222b1e85ba56c55036bca97bd635289f`)

## Native harness

- `examples/benchmark_ntok` (Rust): preloads files, encodes raw bytes with `encode_bytes`,
  counts tokens, verifies `decode(encode(x)) == x`, deterministic across repeats.
- For this report: `--repeats 1 --warmup 0` (token counts are deterministic; throughput was recorded separately and is **not** cross-compared with API-based tokenizers).

## Reference tokenizers

- o200k_base: `tiktoken 0.14.0` (`tiktoken.get_encoding('o200k_base')`), rank file downloaded by tiktoken at first use; `allowed_special=set()`.
- Qwen: `tokenizers` 0.23.3 on the pinned HF snapshot below.

## Date and runtimes

- Measurement date: 2026-11-16 (UTC)
- o200k_base full pass: ~10 s
- Qwen2.5-7B-Instruct full pass: ~82 s
- Native NTokenizer passes: seconds each
