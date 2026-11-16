# Security Policy

## Reporting a vulnerability

If you discover a security issue in NTokenizer — for example, memory unsafety in the
C ABI, a parser crash in the model loader, or a data-integrity flaw in the `.ntok`
format — please report it **privately** by opening a security advisory on GitHub
("Report a vulnerability" in the Security tab) or by opening a minimal public issue
that describes the class of problem without exploit details.

Do **not** include exploits, malicious model files, or private data in public issues.

## Supported versions

| Version | Supported |
|---|---|
| v0.3-polished (current) | Yes |
| v0.2 / v0.1 | No — upgrade |

## Security posture

- The decoder never fabricates bytes: it reconstructs exactly the bytes stored in
  token entries, or returns an error for unknown IDs.
- Model files carry CRC32 checksums and a payload length; the loader rejects corrupt
  or truncated files (see `src/serialization/format.rs`) and is covered by
  `tests/serialization_security_tests.rs`.
- Atomic save uses temp-file + rename.
- Fuzzing (`fuzz/roundtrip`, `fuzz/load_model`) ran ~1.0M combined runs without
  crashes. ASAN-instrumented fuzzing is not yet available in this project's build
  environment and is tracked as an open item — it is not claimed as done.
- This project does not handle credentials, tokens, or secrets at runtime; never put
  secrets in issues, models, or logs.

## Scope notes

- The `.ntok` format executes no code; it contains only data tables.
- Python bindings use ctypes over a C ABI; treat loaded models as untrusted input and
  validate hashes before loading in production.
