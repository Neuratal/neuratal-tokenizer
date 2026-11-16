# Python Integration

Neuratal Tokenizer ships zero-third-party-dependency Python bindings (stdlib `ctypes`
only). The heavy lifting (BPE training, encoding, decoding, serialization) stays
in Rust; Python passes bytes/strings across the C ABI and owns no tokenization
logic itself.

## Requirements

- Python 3.8+
- The native library (`libneuratal_tokenizer.so`, or `.dylib`/`.dll` on other platforms)
  built with Rust: `cargo build --release`

The Python module locates the library in this order:

1. `NTOKENIZER_LIB` environment variable (explicit path, useful for tests)
2. `<repo>/target/release/libneuratal_tokenizer.so`
3. `<repo>/target/debug/libneuratal_tokenizer.so`
4. `libneuratal_tokenizer.so` on the system library search path

## API

```python
import os
os.environ["NTOKENIZER_LIB"] = "/path/to/libneuratal_tokenizer.so"  # optional

from neuratal_tokenizer import Tokenizer, TokenizerError

# Load a trained model or start from the 256 byte tokens
tokenizer = Tokenizer.from_file("tokenizer.ntok")   # or Tokenizer.base()

ids = tokenizer.encode("Hello, world!")           # list[int]
text = tokenizer.decode(ids)                      # str (strict UTF-8)

batch = tokenizer.encode_batch([                  # list[list[int]]
    "Hello",
    "Multilingual text",
    "Mathematics: x + y = 10",
])

print(ids)
print(text)
print(tokenizer.vocab_size)                       # int
print(tokenizer.special_tokens)                   # {'<|bos|>': 16380, ...}
print(tokenizer.special_token_id("<|bos|>"))      # int or None
print(len(tokenizer))                             # vocab_size
print(repr(tokenizer))                            # <Neuratal Tokenizer vocab_size=...>
```

### Raw bytes

```python
ids = tokenizer.encode_bytes(b"\x00\xff invalid \x80 utf-8")
data = tokenizer.decode_bytes(ids)                # exact bytes back
```

The text API is strict: `decode` raises `TokenizerError` if the reconstructed
byte stream is not valid UTF-8. `decode_bytes` never validates and always
returns the exact bytes.

### Errors

All failures raise `TokenizerError` (a subclass of `RuntimeError`):

- `Tokenizer.from_file(...)` on missing/corrupt/incompatible models
- `decode` on invalid UTF-8 or unknown token IDs
- `encode`/`decode_bytes` on native library failures (should not happen in practice)

Invalid argument types and non-sequence IDs raise `TypeError`. Token IDs are
bulk-converted to C `uint32_t` via `array.array`; out-of-range values raise
`TypeError` from that conversion. Note `True` is accepted as token 1 (it is an
`int` subclass).

### Lifecycle

```python
with Tokenizer.from_file("tokenizer.ntok") as tok:
    ids = tok.encode("scoped")
# native handle is freed here

tokenizer.close()   # explicit free (idempotent); using it afterwards raises
```

## Implementation notes

- `encode` uses `neuratal_tokenizer_encode_text_len` (byte-counted, not NUL-terminated),
  so embedded NUL bytes round-trip exactly.
- Token IDs cross the boundary as bulk C arrays (`array.array('I', ...)`), avoiding
  per-item Python loops; Rust-allocated ID buffers are converted back with a native
  pointer slice.
- Rust-allocated buffers are freed exactly once per call, in a `finally` block,
  with the matching `neuratal_tokenizer_free_*` function.
- Release and debug FFI builds expose the same ABI.
- The bindings call `catch_unwind`-guarded entry points; a panic inside Rust
  surfaces as a `TokenizerError`, never as undefined behaviour across the ABI.

## Measured A/B: ctypes (shipping) vs PyO3 (optional probe)

Measured on this machine, base tokenizer, single thread:

| Path | Short call (µs) | 105 KB encode | 105 KB decode |
|------|---:|---:|---:|
| ctypes (optimized wrapper) | 3.58 | ~24.5 MB/s | ~93.5 MB/s |
| PyO3 (`--features pyo3` probe) | 0.58 | ~70 MB/s | ~49 MB/s |

(PyO3 numbers are from the minimal `_neuratal_tokenizer` probe module; the
ctypes wrapper was first optimized — `array.array`-based ID validation and
native pointer slices — before this comparison. PyO3 wins short-call overhead
and mid-size encodes; ctypes wins big-payload decode; neither dominates.)

**Decision: the ctypes bindings remain the shipping path.** Reasons:

1. They are the tested, working integration (14 integration tests pass).
2. Zero third-party Python dependencies; build requires no Python headers.
3. PyO3 would add ~10 crates (incl. `target-lexicon` under Apache-2.0 WITH
   LLVM-exception) and a compiled-module distribution story.
4. At realistic tokenizer workload sizes (KB–MB texts), absolute differences
   are small next to model/training work.

The PyO3 probe remains in the tree as an optional cargo feature (`--features pyo3`)
for teams that measure otherwise in their pipeline. It is not built by default
and is not a supported public API. If you want to ship a PyO3 module instead,
the benchmark commands are above; re-measure on your hardware.

## Verified vs unverified

- **Verified** (automated, `tests/test_python_bindings.py`): loading from disk,
  encode/decode/batch, NUL bytes, embedded-NUL round-trip, invalid input,
  repeated calls, context manager, `special_tokens`, trained-model loads.
- **Verified** (manual example run, torch 2.15.0+cpu): `examples/pytorch_training_example.py`
  tokenizes, pads with `<|pad|>`, builds `torch` tensors, runs a forward pass.
  It is an integration example, not a training framework.
- **Not verified**: Windows/macOS dynamic loading (Linux only here), GPU-side
  tensor pipelines, multi-threaded Python callers sharing one tokenizer handle
  (the Rust API is `Sync`; the ctypes wrapper is exercised single-threaded
  only).
