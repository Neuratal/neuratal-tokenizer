#!/usr/bin/env python3
"""
Python integration tests for Neuratal Tokenizer.

Covers (Phase 5D/5F):
- Loading a base tokenizer and a trained .ntok model from disk (via the ntokenizer CLI)
- Encoding / decoding / batch operations
- Vocabulary size access and special-token access
- Handling of NUL bytes and invalid UTF-8
- Input validation and clear exceptions
- Repeated calls (memory-ownership stability)
- Context manager / explicit close semantics

Runs standalone (`python3 tests/test_python_bindings.py`) or under pytest
(`pytest tests/test_python_bindings.py`) if pytest is installed.
"""

import atexit
import os
import shutil
import subprocess
import sys
import tempfile

# Add python module path
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "python")))

from neuratal_tokenizer import Tokenizer, TokenizerError, native_version

REPO_ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))


def _find_cli():
    for candidate in (
        os.path.join(REPO_ROOT, "target", "release", "ntokenizer"),
        os.path.join(REPO_ROOT, "target", "debug", "ntokenizer"),
    ):
        if os.path.exists(candidate) and os.access(candidate, os.X_OK):
            return candidate
    return None


# ----------------------------------------------------------------------
# Module-scope fixture: train a tiny model once for the from_file tests
# ----------------------------------------------------------------------
_TRAINED_MODEL_PATH = None
_TRAINED_WORKSPACE = None


def _trained_model_path():
    global _TRAINED_MODEL_PATH, _TRAINED_WORKSPACE
    if _TRAINED_MODEL_PATH:
        return _TRAINED_MODEL_PATH

    cli = _find_cli()
    if cli is None:
        raise RuntimeError(
            "ntokenizer CLI binary not found; build with `cargo build --release` first"
        )

    workspace = tempfile.mkdtemp(prefix="ntok_py_test_")
    corpus_path = os.path.join(workspace, "corpus.txt")
    with open(corpus_path, "w", encoding="utf-8") as f:
        for _ in range(8):
            f.write("the quick brown fox jumps over the lazy dog\n")
            f.write("machine learning models require large amounts of text data\n")
            f.write("numbers 1234567890 and formulas e^{iπ} + 1 = 0\n")
            f.write("multilingual Türkçe Русский العربية 中文 한국어 test\n")

    model_path = os.path.join(workspace, "trained.ntok")
    subprocess.run(
        [
            cli,
            "train",
            "--input", corpus_path,
            "--vocab-size", "300",
            "--min-freq", "2",
            "--output", model_path,
            "--seed", "42",
            "--pretokenizer", "conservative",
        ],
        check=True,
        capture_output=True,
    )
    _TRAINED_MODEL_PATH = model_path
    _TRAINED_WORKSPACE = workspace
    return model_path


def _cleanup_trained_model():
    global _TRAINED_WORKSPACE
    if _TRAINED_WORKSPACE:
        shutil.rmtree(_TRAINED_WORKSPACE, ignore_errors=True)
        _TRAINED_WORKSPACE = None


atexit.register(_cleanup_trained_model)


# ----------------------------------------------------------------------
# Base tokenizer tests
# ----------------------------------------------------------------------
def test_base_tokenizer():
    tok = Tokenizer.base()
    assert tok.vocab_size == 256, f"Expected 256 base bytes, got {tok.vocab_size}"
    assert len(tok) == 256

    text = "Hello world! 🚀 Türkçe metin: Şemsi Paşa Pasajı. ∑ = 42"
    tokens = tok.encode(text)
    decoded = tok.decode(tokens)
    assert decoded == text, f"Decoded text {decoded!r} does not match {text!r}"

    raw_data = b"\x00\xFF\xFE\x80Hello"
    byte_tokens = tok.encode_bytes(raw_data)
    decoded_raw = tok.decode_bytes(byte_tokens)
    assert decoded_raw == raw_data, "Binary roundtrip failed"

    batch = ["Hello world", "Merhaba dünya", "Bonjour le monde"]
    batch_tokens = tok.encode_batch(batch)
    batch_decoded = tok.decode_batch(batch_tokens)
    assert batch_decoded == batch, "Batch roundtrip failed"


def test_text_with_embedded_nul_bytes():
    tok = Tokenizer.base()
    text = "a\x00b\x00c"
    ids = tok.encode(text)
    # Base tokenizer maps each byte to one ID; the NUL must survive as token 0.
    assert ids[0] == ord("a")
    assert ids[1] == 0
    assert ids[2] == ord("b")

    decoded = tok.decode(ids)
    assert decoded == text, "NUL-containing text must round-trip exactly"
    assert tok.decode_bytes(ids) == b"a\x00b\x00c"


def test_empty_inputs():
    tok = Tokenizer.base()
    assert tok.encode("") == []
    assert tok.decode([]) == ""
    assert tok.decode_bytes([]) == b""
    assert tok.encode_bytes(b"") == []
    assert tok.encode_batch([]) == []
    assert tok.decode_batch([]) == []


def test_invalid_id_values():
    tok = Tokenizer.base()
    # bool is an int subclass; True is accepted as token 1 (documented).
    # Range violations and non-int values raise TypeError/ValueError.
    for bad in ([-1], [0x100000000], [1.5], ["1"], [None], [3 + 4j]):
        try:
            tok.decode(bad)
        except (TypeError, ValueError, TokenizerError):
            pass
        else:
            raise AssertionError(f"decode({bad!r}) should have raised")

    try:
        tok.decode([999999])
    except TokenizerError:
        pass
    else:
        raise AssertionError("decode of unknown token id should raise TokenizerError")


def test_encode_rejects_non_string():
    tok = Tokenizer.base()
    for bad in (123, None, b"bytes-not-str", ["list"]):
        try:
            tok.encode(bad)
        except TypeError:
            pass
        else:
            raise AssertionError(f"encode({bad!r}) should have raised TypeError")


def test_special_tokens_base_tokenizer():
    tok = Tokenizer.base()
    assert tok.special_tokens == {}
    assert tok.special_token_id("<|bos|>") is None


def test_repeated_calls_memory_stability():
    tok = Tokenizer.base()
    text = "memory ownership test with repeated encode/decode cycles 🚀" * 8
    for _ in range(300):
        ids = tok.encode(text)
        assert tok.decode(ids) == text
        tok.encode_bytes(text.encode("utf-8"))
        tok.special_tokens


def test_context_manager_and_close():
    with Tokenizer.base() as tok:
        ids = tok.encode("hello")
        assert tok.decode(ids) == "hello"
    try:
        tok.encode("after close")
    except TokenizerError:
        pass
    else:
        raise AssertionError("operations after close() must raise TokenizerError")
    # Double close must be safe
    tok.close()


def test_native_version():
    version = native_version()
    assert isinstance(version, str) and version.count(".") >= 1, version


def test_repr():
    tok = Tokenizer.base()
    assert "vocab_size=256" in repr(tok)


# ----------------------------------------------------------------------
# Trained model (from_file) tests
# ----------------------------------------------------------------------
def test_from_file_trained_model():
    model = _trained_model_path()
    tok = Tokenizer.from_file(model)
    assert tok.vocab_size > 256
    assert tok.vocab_size == len(tok)

    text = "the quick brown fox jumps"
    ids = tok.encode(text)
    assert ids, "trained model should encode text into at least one token"
    assert tok.decode(ids) == text

    batch = ["the quick brown fox", "numbers 1234567890", "Türkçe Русский 中文"]
    batch_ids = tok.encode_batch(batch)
    assert len(batch_ids) == len(batch)
    assert tok.decode_batch(batch_ids) == batch


def test_from_file_special_tokens():
    model = _trained_model_path()
    tok = Tokenizer.from_file(model)

    specials = tok.special_tokens
    for expected in ("<|bos|>", "<|eos|>", "<|pad|>", "<|unk|>", "<|system|>", "<|user|>", "<|assistant|>"):
        assert expected in specials, f"missing special token {expected}"

    bos_id = tok.special_token_id("<|bos|>")
    assert isinstance(bos_id, int)
    decoded = tok.decode([bos_id])
    assert "<|bos|>" in decoded


def test_from_file_invalid_model():
    with tempfile.NamedTemporaryFile(suffix=".ntok", delete=False) as f:
        f.write(b"definitely not a valid ntok model file" * 4)
        bad_path = f.name
    try:
        try:
            Tokenizer.from_file(bad_path)
        except TokenizerError:
            pass
        else:
            raise AssertionError("from_file on invalid model must raise TokenizerError")
    finally:
        os.unlink(bad_path)


def test_from_file_missing_model():
    try:
        Tokenizer.from_file("/nonexistent/path/to/model.ntok")
    except TokenizerError:
        pass
    else:
        raise AssertionError("from_file on missing file must raise TokenizerError")


# ----------------------------------------------------------------------
# Minimal runner for non-pytest environments
# ----------------------------------------------------------------------
def _run_all():
    tests = [(name, fn) for name, fn in sorted(globals().items()) if name.startswith("test_") and callable(fn)]
    failures = []
    for name, fn in tests:
        try:
            fn()
            print(f"PASS {name}")
        except Exception as exc:  # noqa: BLE001 - report and continue
            failures.append((name, exc))
            print(f"FAIL {name}: {exc}")
    _cleanup_trained_model()
    if failures:
        print(f"\n{len(failures)} test(s) FAILED")
        return 1
    print(f"\nAll {len(tests)} Python integration tests PASSED successfully!")
    return 0


if __name__ == "__main__":
    sys.exit(_run_all())
