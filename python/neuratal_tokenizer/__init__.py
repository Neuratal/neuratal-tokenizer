"""
Neuratal Tokenizer Python Integration
==============================

Zero-third-party-dependency bindings (stdlib ``ctypes`` only) for the
Neuratal Tokenizer Rust library.

Example::

    from neuratal_tokenizer import Tokenizer

    tokenizer = Tokenizer.from_file("tokenizer.ntok")
    ids = tokenizer.encode("Hello, world!")
    text = tokenizer.decode(ids)
    batch = tokenizer.encode_batch(["Hello", "Multilingual text", "Mathematics: x + y = 10"])
"""

import array
import ctypes
import json
import os
from typing import Dict, List, Optional


class TokenizerError(RuntimeError):
    """Raised when a tokenizer operation fails (invalid model, invalid input, or FFI error)."""


def _find_library() -> str:
    # Explicit override wins (useful for tests and custom layouts)
    env_path = os.environ.get("NTOKENIZER_LIB")
    if env_path:
        return env_path

    base_dir = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
    candidates = [
        os.path.join(base_dir, "target", "release", "libneuratal_tokenizer.so"),
        os.path.join(base_dir, "target", "debug", "libneuratal_tokenizer.so"),
        os.path.join(base_dir, "libneuratal_tokenizer.so"),
    ]
    for c in candidates:
        if os.path.exists(c):
            return c
    # Fall back to the system library search path
    return "libneuratal_tokenizer.so"


def _u32_array_to_list(ptr, n: int) -> List[int]:
    """Fast native conversion of a raw uint32 buffer (Rust-owned) to a Python list."""
    if n == 0:
        return []
    return ptr[:n]


class _NTokenizerLib:
    REQUIRED_SYMBOLS = [
        "neuratal_tokenizer_new_base",
        "neuratal_tokenizer_load",
        "neuratal_tokenizer_free",
        "neuratal_tokenizer_vocab_size",
        "neuratal_tokenizer_version",
        "neuratal_tokenizer_encode_text_len",
        "neuratal_tokenizer_decode_bytes",
        "neuratal_tokenizer_free_tokens",
        "neuratal_tokenizer_free_string",
        "neuratal_tokenizer_free_bytes",
        "neuratal_tokenizer_encode_bytes",
        "neuratal_tokenizer_special_tokens_json",
    ]

    def __init__(self):
        lib_path = _find_library()
        try:
            self._lib = ctypes.CDLL(lib_path)
        except OSError as exc:
            raise TokenizerError(
                f"Failed to load Neuratal Tokenizer native library at {lib_path!r}: {exc}. "
                "Build it with `cargo build --release` or set NTOKENIZER_LIB."
            ) from exc

        missing = [s for s in self.REQUIRED_SYMBOLS if not hasattr(self._lib, s)]
        if missing:
            raise TokenizerError(
                f"Neuratal Tokenizer native library at {lib_path!r} is missing symbols: {missing}. "
                "Rebuild with a current source tree."
            )

        # neuratal_tokenizer_new_base() -> void*
        self._lib.neuratal_tokenizer_new_base.argtypes = []
        self._lib.neuratal_tokenizer_new_base.restype = ctypes.c_void_p

        # neuratal_tokenizer_load(const char*) -> void*
        self._lib.neuratal_tokenizer_load.argtypes = [ctypes.c_char_p]
        self._lib.neuratal_tokenizer_load.restype = ctypes.c_void_p

        # neuratal_tokenizer_free(void*)
        self._lib.neuratal_tokenizer_free.argtypes = [ctypes.c_void_p]
        self._lib.neuratal_tokenizer_free.restype = None

        # neuratal_tokenizer_vocab_size(void*) -> uint32_t
        self._lib.neuratal_tokenizer_vocab_size.argtypes = [ctypes.c_void_p]
        self._lib.neuratal_tokenizer_vocab_size.restype = ctypes.c_uint32

        # neuratal_tokenizer_version() -> const char* (static, do not free)
        self._lib.neuratal_tokenizer_version.argtypes = []
        self._lib.neuratal_tokenizer_version.restype = ctypes.c_char_p

        # neuratal_tokenizer_encode_text_len(void*, const char*, size_t, size_t*) -> uint32_t*
        self._lib.neuratal_tokenizer_encode_text_len.argtypes = [
            ctypes.c_void_p,
            ctypes.c_char_p,
            ctypes.c_size_t,
            ctypes.POINTER(ctypes.c_size_t),
        ]
        self._lib.neuratal_tokenizer_encode_text_len.restype = ctypes.POINTER(ctypes.c_uint32)

        # neuratal_tokenizer_encode_bytes(void*, const uint8_t*, size_t, size_t*) -> uint32_t*
        self._lib.neuratal_tokenizer_encode_bytes.argtypes = [
            ctypes.c_void_p,
            ctypes.c_char_p,  # raw byte buffer (reinterpreted as char*)
            ctypes.c_size_t,
            ctypes.POINTER(ctypes.c_size_t),
        ]
        self._lib.neuratal_tokenizer_encode_bytes.restype = ctypes.POINTER(ctypes.c_uint32)

        # neuratal_tokenizer_decode_bytes(void*, const uint32_t*, size_t, size_t*) -> uint8_t*
        self._lib.neuratal_tokenizer_decode_bytes.argtypes = [
            ctypes.c_void_p,
            ctypes.POINTER(ctypes.c_uint32),
            ctypes.c_size_t,
            ctypes.POINTER(ctypes.c_size_t),
        ]
        self._lib.neuratal_tokenizer_decode_bytes.restype = ctypes.POINTER(ctypes.c_uint8)

        # neuratal_tokenizer_free_tokens(uint32_t*, size_t)
        self._lib.neuratal_tokenizer_free_tokens.argtypes = [
            ctypes.POINTER(ctypes.c_uint32),
            ctypes.c_size_t,
        ]
        self._lib.neuratal_tokenizer_free_tokens.restype = None

        # neuratal_tokenizer_free_bytes(uint8_t*, size_t)
        self._lib.neuratal_tokenizer_free_bytes.argtypes = [
            ctypes.POINTER(ctypes.c_uint8),
            ctypes.c_size_t,
        ]
        self._lib.neuratal_tokenizer_free_bytes.restype = None

        # neuratal_tokenizer_free_string(char*)
        self._lib.neuratal_tokenizer_free_string.argtypes = [ctypes.c_void_p]
        self._lib.neuratal_tokenizer_free_string.restype = None

        # neuratal_tokenizer_special_tokens_json(void*) -> char*
        self._lib.neuratal_tokenizer_special_tokens_json.argtypes = [ctypes.c_void_p]
        self._lib.neuratal_tokenizer_special_tokens_json.restype = ctypes.c_void_p


_LIB = None


def _get_lib() -> _NTokenizerLib:
    global _LIB
    if _LIB is None:
        _LIB = _NTokenizerLib()
    return _LIB


def native_version() -> str:
    """Returns the version string of the linked native library."""
    return _get_lib()._lib.neuratal_tokenizer_version().decode("utf-8")


try:
    __version__ = _get_lib()._lib.neuratal_tokenizer_version().decode("utf-8")
except TokenizerError:
    # Library not built/loadable yet; import of the module still succeeds so
    # callers get a clear error at first use instead of at import time.
    __version__ = "unknown"


class Tokenizer:
    """High-performance multilingual byte-level BPE tokenizer."""

    def __init__(self, raw_ptr: int):
        if not raw_ptr:
            raise TokenizerError("Invalid null tokenizer pointer")
        self._ptr = raw_ptr
        self._lib = _get_lib()
        self._closed = False

    @classmethod
    def base(cls) -> "Tokenizer":
        """Creates a base tokenizer with 256 byte tokens."""
        lib = _get_lib()
        ptr = lib._lib.neuratal_tokenizer_new_base()
        if not ptr:
            raise TokenizerError("Failed to create base tokenizer")
        return cls(ptr)

    @classmethod
    def from_file(cls, path: str) -> "Tokenizer":
        """Loads a tokenizer model from an `.ntok` file."""
        lib = _get_lib()
        c_path = os.fsencode(path)
        ptr = lib._lib.neuratal_tokenizer_load(c_path)
        if not ptr:
            raise TokenizerError(
                f"Failed to load tokenizer from {path!r}: file missing, corrupted, or incompatible"
            )
        return cls(ptr)

    # ------------------------------------------------------------------
    # Lifecycle
    # ------------------------------------------------------------------
    def close(self) -> None:
        """Frees the underlying native tokenizer. Idempotent."""
        if getattr(self, "_ptr", None):
            self._lib._lib.neuratal_tokenizer_free(self._ptr)
            self._ptr = None
        self._closed = True

    def __del__(self):
        try:
            self.close()
        except Exception:
            pass

    def __enter__(self) -> "Tokenizer":
        return self

    def __exit__(self, exc_type, exc, tb) -> None:
        self.close()

    def _check_open(self) -> int:
        if getattr(self, "_closed", False) or not getattr(self, "_ptr", None):
            raise TokenizerError("Tokenizer has been closed")
        return self._ptr

    # ------------------------------------------------------------------
    # Properties
    # ------------------------------------------------------------------
    @property
    def vocab_size(self) -> int:
        """Returns the total vocabulary size."""
        return int(self._lib._lib.neuratal_tokenizer_vocab_size(self._check_open()))

    def __len__(self) -> int:
        return self.vocab_size

    @property
    def special_tokens(self) -> Dict[str, int]:
        """Returns a dict mapping special token content strings to their IDs."""
        ptr = self._lib._lib.neuratal_tokenizer_special_tokens_json(self._check_open())
        if not ptr:
            raise TokenizerError("Failed to read special tokens from native library")
        try:
            raw = ctypes.string_at(ptr).decode("utf-8")
        finally:
            self._lib._lib.neuratal_tokenizer_free_string(ptr)
        items = json.loads(raw)
        return {item["content"]: int(item["id"]) for item in items}

    def special_token_id(self, content: str) -> Optional[int]:
        """Returns the ID of a special token by content, or None if not registered."""
        return self.special_tokens.get(content)

    # ------------------------------------------------------------------
    # Encode / decode
    # ------------------------------------------------------------------
    def encode(self, text: str) -> List[int]:
        """Encodes a UTF-8 text string into token IDs."""
        if not isinstance(text, str):
            raise TypeError(f"encode() expects str, got {type(text).__name__}")
        ptr = self._check_open()
        data = text.encode("utf-8")
        out_len = ctypes.c_size_t()
        tokens_ptr = self._lib._lib.neuratal_tokenizer_encode_text_len(
            ptr, data, len(data), ctypes.byref(out_len)
        )
        if not tokens_ptr:
            raise TokenizerError("Failed to encode text (invalid UTF-8 input or native error)")
        try:
            return _u32_array_to_list(tokens_ptr, out_len.value)
        finally:
            self._lib._lib.neuratal_tokenizer_free_tokens(tokens_ptr, out_len.value)

    def encode_bytes(self, data: bytes) -> List[int]:
        """Encodes raw bytes (supports invalid UTF-8)."""
        if not isinstance(data, (bytes, bytearray, memoryview)):
            raise TypeError(f"encode_bytes() expects bytes-like, got {type(data).__name__}")
        ptr = self._check_open()
        buf = bytes(data)
        out_len = ctypes.c_size_t()
        tokens_ptr = self._lib._lib.neuratal_tokenizer_encode_bytes(
            ptr, buf, len(buf), ctypes.byref(out_len)
        )
        if not tokens_ptr:
            raise TokenizerError("Failed to encode bytes")
        try:
            return _u32_array_to_list(tokens_ptr, out_len.value)
        finally:
            self._lib._lib.neuratal_tokenizer_free_tokens(tokens_ptr, out_len.value)

    def decode(self, ids: List[int]) -> str:
        """Decodes token IDs into a strict UTF-8 string.

        Raises TokenizerError if the reconstructed byte stream is not valid UTF-8
        (use decode_bytes for lossy-tolerant byte handling).
        """
        raw = self._decode_to_bytes(ids)
        try:
            return raw.decode("utf-8")
        except UnicodeDecodeError as exc:
            raise TokenizerError(
                f"Decoded byte sequence is not valid UTF-8: {exc}. "
                "Use decode_bytes() for arbitrary byte output."
            ) from exc

    def decode_bytes(self, ids: List[int]) -> bytes:
        """Decodes token IDs back into raw bytes (supports invalid UTF-8 output)."""
        return self._decode_to_bytes(ids)

    def _decode_to_bytes(self, ids: List[int]) -> bytes:
        ptr = self._check_open()
        ids_arr = self._validate_ids(ids)
        ids_buf = ids_arr.buffer_info()[0]
        c_ids = ctypes.cast(ids_buf, ctypes.POINTER(ctypes.c_uint32))
        out_len = ctypes.c_size_t()
        bytes_ptr = self._lib._lib.neuratal_tokenizer_decode_bytes(
            ptr, c_ids, len(ids_arr), ctypes.byref(out_len)
        )
        if not bytes_ptr:
            raise TokenizerError(
                "Failed to decode token IDs (unknown token ID for this vocabulary)"
            )
        try:
            return ctypes.string_at(bytes_ptr, out_len.value)
        finally:
            self._lib._lib.neuratal_tokenizer_free_bytes(bytes_ptr, out_len.value)

    @staticmethod
    def _validate_ids(ids: List[int]) -> "array.array":
        if not isinstance(ids, (list, tuple)):
            raise TypeError(f"ids must be a list or tuple of ints, got {type(ids).__name__}")
        # C-speed bulk conversion to uint32 with type and range validation.
        try:
            arr = array.array("I", ids)
        except (TypeError, ValueError, OverflowError) as exc:
            raise TypeError(f"ids must be ints in uint32 range [0, 4294967295]: {exc}") from exc
        if arr.itemsize != 4:
            # Defensive: platform without 4-byte unsigned int; fall back to explicit check.
            raise RuntimeError(f"array 'I' itemsize is {arr.itemsize}, expected 4")
        return arr

    def encode_batch(self, texts: List[str]) -> List[List[int]]:
        """Encodes a batch of texts."""
        return [self.encode(t) for t in texts]

    def decode_batch(self, batches: List[List[int]]) -> List[str]:
        """Decodes a batch of token ID lists."""
        return [self.decode(b) for b in batches]

    def __repr__(self) -> str:
        state = "closed" if getattr(self, "_closed", False) or not getattr(self, "_ptr", None) else f"vocab_size={self.vocab_size}"
        return f"<Neuratal Tokenizer {state}>"


__all__ = ["Tokenizer", "TokenizerError", "native_version"]
