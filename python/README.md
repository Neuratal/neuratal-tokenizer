# NTokenizer Python Package

Zero-third-party-dependency Python bindings for the Neuratal Tokenizer (NTokenizer)
Rust engine (stdlib `ctypes` only).

## Install (from a source checkout)

```bash
# 1. Build the native library
cargo build --release

# 2. Either add the package directory to PYTHONPATH…
PYTHONPATH=$PWD/python python3 -c "from neuratal_tokenizer import Tokenizer; print(Tokenizer.base().vocab_size)"

# 3. …or install it (use a virtualenv)
pip install -e python/
```

The package locates the native library in this order:

1. `$NTOKENIZER_LIB` (explicit path),
2. `<repo>/target/release/libneuratal_tokenizer.so`,
3. `<repo>/target/debug/libneuratal_tokenizer.so`,
4. `libneuratal_tokenizer.so` on the system library search path.

## Usage

```python
from neuratal_tokenizer import Tokenizer

tokenizer = Tokenizer.from_file("model.ntok")    # or Tokenizer.base()
ids = tokenizer.encode("Hello, world!")
text = tokenizer.decode(ids)
batch = tokenizer.encode_batch(["Hello", "多语言文本", "Mathematics: x + y = 10"])
print(ids)
print(text)
```

See `docs/python_integration.md` for the full API, FFI details, and the
ctypes-vs-PyO3 benchmark comparison. See `examples/pytorch_training_example.py`
for a PyTorch pipeline example.

## Tests

```bash
python3 tests/test_python_bindings.py
```

(requires the release CLI at `target/release/ntokenizer`, built by `cargo build --release`)
