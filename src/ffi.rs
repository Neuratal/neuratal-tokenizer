//! C-compatible Foreign Function Interface (FFI) for Python and C/C++ integration.
//!
//! Provides memory-safe C ABI endpoints for loading models, encoding, and decoding.
//!
//! Safety contract for all exported functions:
//! - Null pointers are checked before dereferencing; on any failure (or panic inside
//!   the library) a null pointer / zero is returned instead of unwinding across the
//!   C boundary.
//! - Returned buffers are heap-allocated by the library and MUST be released with the
//!   matching `neuratal_tokenizer_free_*` function, exactly once, with the same length for
//!   token arrays and byte buffers.
//! - The `Tokenizer*` pointer is opaque; it must be created by `neuratal_tokenizer_new_base`
//!   or `neuratal_tokenizer_load` and released with `neuratal_tokenizer_free`.

use crate::Tokenizer;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::panic::{AssertUnwindSafe, catch_unwind};

/// Runs `f` and converts any panic into `fallback`, so unwinding never crosses the C ABI.
#[inline]
fn guard<T>(f: impl FnOnce() -> T, fallback: T) -> T {
    catch_unwind(AssertUnwindSafe(f)).unwrap_or(fallback)
}

/// Creates a new base tokenizer with 256 byte tokens.
#[unsafe(no_mangle)]
///
/// # Safety
/// No parameters. The returned pointer is heap-allocated and must be freed exactly once with `neuratal_tokenizer_free`.
pub unsafe extern "C" fn neuratal_tokenizer_new_base() -> *mut Tokenizer {
    guard(
        || Box::into_raw(Box::new(Tokenizer::new_base())),
        std::ptr::null_mut(),
    )
}

/// Loads a tokenizer model from a file path (.ntok).
///
/// Returns null pointer on error.
#[unsafe(no_mangle)]
///
/// # Safety
/// `path` must be null or a valid NUL-terminated C string readable for the duration of the call.
/// The returned pointer (or null on failure) must be freed with `neuratal_tokenizer_free`.
pub unsafe extern "C" fn neuratal_tokenizer_load(path: *const c_char) -> *mut Tokenizer {
    guard(
        || {
            if path.is_null() {
                return std::ptr::null_mut();
            }
            let c_str = unsafe { CStr::from_ptr(path) };
            let path_str = match c_str.to_str() {
                Ok(s) => s,
                Err(_) => return std::ptr::null_mut(),
            };

            match Tokenizer::load_from_file(path_str) {
                Ok(tok) => Box::into_raw(Box::new(tok)),
                Err(_) => std::ptr::null_mut(),
            }
        },
        std::ptr::null_mut(),
    )
}

/// Frees a Tokenizer instance.
#[unsafe(no_mangle)]
///
/// # Safety
/// `ptr` must be null, or a pointer previously returned by `neuratal_tokenizer_new_base`/`neuratal_tokenizer_load` that has not already been freed.
/// After this call the pointer must not be used again.
pub unsafe extern "C" fn neuratal_tokenizer_free(ptr: *mut Tokenizer) {
    guard(
        || {
            if !ptr.is_null() {
                unsafe {
                    drop(Box::from_raw(ptr));
                }
            }
        },
        (),
    )
}

/// Returns the vocabulary size of the tokenizer.
#[unsafe(no_mangle)]
///
/// # Safety
/// `ptr` must be null (returns 0) or a valid pointer to a live Tokenizer created by this library.
pub unsafe extern "C" fn neuratal_tokenizer_vocab_size(ptr: *const Tokenizer) -> u32 {
    guard(
        || {
            if ptr.is_null() {
                return 0;
            }
            let tok = unsafe { &*ptr };
            tok.vocab_size() as u32
        },
        0,
    )
}

/// Returns library version string, e.g. "0.1.0". The returned pointer is static and
/// must NOT be freed.
#[unsafe(no_mangle)]
///
/// # Safety
/// No parameters. The returned pointer references a static string that is always valid and must not be freed.
pub unsafe extern "C" fn neuratal_tokenizer_version() -> *const c_char {
    concat!(env!("CARGO_PKG_VERSION"), "\0").as_ptr() as *const c_char
}

/// Encodes a UTF-8 text string into a buffer of token IDs.
///
/// The text is read as a NUL-terminated C string; strings containing embedded
/// NUL bytes will be truncated at the first NUL. Prefer `neuratal_tokenizer_encode_text_len`
/// for binary-safe text passing.
///
/// Returns a pointer to a heap-allocated array of `u32`, and writes its length to `out_len`.
/// The caller must free the returned array using `neuratal_tokenizer_free_tokens`.
#[unsafe(no_mangle)]
///
/// # Safety
/// `ptr` must be a valid pointer to a live Tokenizer. `text` must be null or a valid NUL-terminated UTF-8 C string.
/// `out_len` must be a valid, aligned, writable `usize`. The returned buffer must be freed with `neuratal_tokenizer_free_tokens(ptr, *out_len)`.
pub unsafe extern "C" fn neuratal_tokenizer_encode_text(
    ptr: *const Tokenizer,
    text: *const c_char,
    out_len: *mut usize,
) -> *mut u32 {
    guard(
        || {
            if ptr.is_null() || text.is_null() || out_len.is_null() {
                return std::ptr::null_mut();
            }
            let tok = unsafe { &*ptr };
            let c_str = unsafe { CStr::from_ptr(text) };
            let text_str = match c_str.to_str() {
                Ok(s) => s,
                Err(_) => return std::ptr::null_mut(),
            };
            into_raw_tokens(tok.encode_text(text_str), out_len)
        },
        std::ptr::null_mut(),
    )
}

/// Binary-safe UTF-8 text encoder: reads exactly `text_len` bytes from `text`.
///
/// Unlike `neuratal_tokenizer_encode_text`, embedded NUL bytes are preserved.
/// The caller must free the returned array using `neuratal_tokenizer_free_tokens`.
#[unsafe(no_mangle)]
///
/// # Safety
/// `ptr` must be a valid pointer to a live Tokenizer. `text` must point to `text_len` readable bytes (or be null with `text_len == 0`) that form valid UTF-8, possibly with embedded NUL bytes.
/// `out_len` must be a valid, aligned, writable `usize`. The returned buffer must be freed with `neuratal_tokenizer_free_tokens(ptr, *out_len)`.
pub unsafe extern "C" fn neuratal_tokenizer_encode_text_len(
    ptr: *const Tokenizer,
    text: *const c_char,
    text_len: usize,
    out_len: *mut usize,
) -> *mut u32 {
    guard(
        || {
            if ptr.is_null() || (text.is_null() && text_len > 0) || out_len.is_null() {
                return std::ptr::null_mut();
            }
            let tok = unsafe { &*ptr };
            let bytes = if text_len > 0 {
                unsafe { std::slice::from_raw_parts(text as *const u8, text_len) }
            } else {
                &[]
            };
            let text_str = match std::str::from_utf8(bytes) {
                Ok(s) => s,
                Err(_) => return std::ptr::null_mut(),
            };
            into_raw_tokens(tok.encode_text(text_str), out_len)
        },
        std::ptr::null_mut(),
    )
}

/// Shared helper: converts an encode result into a raw heap buffer + out_len.
fn into_raw_tokens(
    result: Result<Vec<u32>, crate::TokenizerError>,
    out_len: *mut usize,
) -> *mut u32 {
    match result {
        Ok(mut tokens) => {
            tokens.shrink_to_fit();
            let len = tokens.len();
            let raw_ptr = tokens.as_mut_ptr();
            std::mem::forget(tokens);
            unsafe {
                *out_len = len;
            }
            raw_ptr
        }
        Err(_) => std::ptr::null_mut(),
    }
}

/// Decodes a sequence of token IDs into a null-terminated UTF-8 C string.
///
/// Returns null if the decoded byte sequence contains a NUL byte or is not
/// valid UTF-8. Prefer `neuratal_tokenizer_decode_bytes` for binary-safe decoding.
/// The caller must free the returned string using `neuratal_tokenizer_free_string`.
#[unsafe(no_mangle)]
///
/// # Safety
/// `ptr` must be a valid pointer to a live Tokenizer. `ids` must point to `len` readable `u32` values, or be null with `len == 0`.
/// The returned pointer must be freed with `neuratal_tokenizer_free_string`.
pub unsafe extern "C" fn neuratal_tokenizer_decode_text(
    ptr: *const Tokenizer,
    ids: *const u32,
    len: usize,
) -> *mut c_char {
    guard(
        || {
            if ptr.is_null() || (ids.is_null() && len > 0) {
                return std::ptr::null_mut();
            }
            let tok = unsafe { &*ptr };
            let id_slice = if len > 0 {
                unsafe { std::slice::from_raw_parts(ids, len) }
            } else {
                &[]
            };

            match tok.decode_text(id_slice) {
                Ok(text) => match CString::new(text) {
                    Ok(c_string) => c_string.into_raw(),
                    Err(_) => std::ptr::null_mut(),
                },
                Err(_) => std::ptr::null_mut(),
            }
        },
        std::ptr::null_mut(),
    )
}

/// Frees a token ID array allocated by `neuratal_tokenizer_encode_text*`.
#[unsafe(no_mangle)]
///
/// # Safety
/// `ptr` must be null, or a buffer previously returned by `neuratal_tokenizer_encode_text*`, with exactly the original `len` passed as `len`.
pub unsafe extern "C" fn neuratal_tokenizer_free_tokens(ptr: *mut u32, len: usize) {
    guard(
        || {
            if !ptr.is_null() {
                unsafe {
                    drop(Vec::from_raw_parts(ptr, len, len));
                }
            }
        },
        (),
    )
}

/// Frees a C string allocated by `neuratal_tokenizer_decode_text` / `neuratal_tokenizer_special_tokens_json`.
#[unsafe(no_mangle)]
///
/// # Safety
/// `ptr` must be null or a pointer previously returned by `neuratal_tokenizer_decode_text` or `neuratal_tokenizer_special_tokens_json`.
pub unsafe extern "C" fn neuratal_tokenizer_free_string(ptr: *mut c_char) {
    guard(
        || {
            if !ptr.is_null() {
                unsafe {
                    drop(CString::from_raw(ptr));
                }
            }
        },
        (),
    )
}

/// Encodes raw bytes (supports invalid UTF-8).
///
/// The caller must free the returned array using `neuratal_tokenizer_free_tokens`.
#[unsafe(no_mangle)]
///
/// # Safety
/// `ptr` must be a valid pointer to a live Tokenizer. `bytes` must point to `in_len` readable bytes (or be null with `in_len == 0`).
/// `out_len` must be a valid, aligned, writable `usize`. The returned buffer must be freed with `neuratal_tokenizer_free_tokens(ptr, *out_len)`.
pub unsafe extern "C" fn neuratal_tokenizer_encode_bytes(
    ptr: *const Tokenizer,
    bytes: *const u8,
    in_len: usize,
    out_len: *mut usize,
) -> *mut u32 {
    guard(
        || {
            if ptr.is_null() || (bytes.is_null() && in_len > 0) || out_len.is_null() {
                return std::ptr::null_mut();
            }
            let tok = unsafe { &*ptr };
            let byte_slice = if in_len > 0 {
                unsafe { std::slice::from_raw_parts(bytes, in_len) }
            } else {
                &[]
            };

            let mut tokens = tok.encode_bytes(byte_slice);
            tokens.shrink_to_fit();
            let len = tokens.len();
            let raw_ptr = tokens.as_mut_ptr();
            std::mem::forget(tokens);
            unsafe {
                *out_len = len;
            }
            raw_ptr
        },
        std::ptr::null_mut(),
    )
}

/// Decodes token IDs back to raw bytes.
///
/// The caller must free the returned buffer using `neuratal_tokenizer_free_bytes`.
#[unsafe(no_mangle)]
///
/// # Safety
/// `ptr` must be a valid pointer to a live Tokenizer. `ids` must point to `in_len` readable `u32` values (or be null with `in_len == 0`).
/// `out_len` must be a valid, aligned, writable `usize`. The returned buffer must be freed with `neuratal_tokenizer_free_bytes(ptr, *out_len)`.
pub unsafe extern "C" fn neuratal_tokenizer_decode_bytes(
    ptr: *const Tokenizer,
    ids: *const u32,
    in_len: usize,
    out_len: *mut usize,
) -> *mut u8 {
    guard(
        || {
            if ptr.is_null() || (ids.is_null() && in_len > 0) || out_len.is_null() {
                return std::ptr::null_mut();
            }
            let tok = unsafe { &*ptr };
            let id_slice = if in_len > 0 {
                unsafe { std::slice::from_raw_parts(ids, in_len) }
            } else {
                &[]
            };

            match tok.decode_bytes(id_slice) {
                Ok(mut bytes) => {
                    bytes.shrink_to_fit();
                    let len = bytes.len();
                    let raw_ptr = bytes.as_mut_ptr();
                    std::mem::forget(bytes);
                    unsafe {
                        *out_len = len;
                    }
                    raw_ptr
                }
                Err(_) => std::ptr::null_mut(),
            }
        },
        std::ptr::null_mut(),
    )
}

/// Frees a byte buffer allocated by `neuratal_tokenizer_decode_bytes`.
#[unsafe(no_mangle)]
///
/// # Safety
/// `ptr` must be null, or a buffer previously returned by `neuratal_tokenizer_decode_bytes`, with exactly the original length passed as `len`.
pub unsafe extern "C" fn neuratal_tokenizer_free_bytes(ptr: *mut u8, len: usize) {
    guard(
        || {
            if !ptr.is_null() {
                unsafe {
                    drop(Vec::from_raw_parts(ptr, len, len));
                }
            }
        },
        (),
    )
}

/// Returns the registered special tokens as a JSON array string:
/// `[{"id": 256, "content": "<|bos|>"}, ...]`
///
/// The caller must free the returned string using `neuratal_tokenizer_free_string`.
/// Returns null on error.
#[unsafe(no_mangle)]
///
/// # Safety
/// `ptr` must be null (returns null) or a valid pointer to a live Tokenizer.
/// The returned pointer must be freed with `neuratal_tokenizer_free_string`.
pub unsafe extern "C" fn neuratal_tokenizer_special_tokens_json(
    ptr: *const Tokenizer,
) -> *mut c_char {
    guard(
        || {
            if ptr.is_null() {
                return std::ptr::null_mut();
            }
            let tok = unsafe { &*ptr };
            let items: Vec<serde_json::Value> = tok
                .special_tokens()
                .all_tokens()
                .iter()
                .map(|st| serde_json::json!({ "id": st.id, "content": st.content }))
                .collect();
            match serde_json::to_string(&items) {
                Ok(json) => match CString::new(json) {
                    Ok(s) => s.into_raw(),
                    Err(_) => std::ptr::null_mut(),
                },
                Err(_) => std::ptr::null_mut(),
            }
        },
        std::ptr::null_mut(),
    )
}
