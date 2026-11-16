//! Optional PyO3 bindings, investigated for the Python integration layer.
//!
//! This module is **not enabled by default**. It exists so the A/B overhead of a
//! native PyO3 binding can be measured against the working ctypes bindings
//! (see docs/python_integration.md). It is a minimal functional surface
//! (load, encode, decode, vocab_size), not a supported public API.
#![cfg(feature = "pyo3")]

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

use crate::Tokenizer;

#[pyclass(unsendable)]
pub struct PyTokenizer {
    inner: Tokenizer,
}

#[pymethods]
impl PyTokenizer {
    /// Creates a base tokenizer with 256 byte tokens.
    #[new]
    fn base() -> Self {
        Self {
            inner: Tokenizer::new_base(),
        }
    }

    /// Loads a tokenizer model from an `.ntok` file.
    #[staticmethod]
    fn from_file(path: &str) -> PyResult<Self> {
        Tokenizer::load_from_file(path)
            .map(|t| Self { inner: t })
            .map_err(PyValueError::new_err)
    }

    /// Encodes UTF-8 text into token IDs.
    fn encode(&self, text: &str) -> PyResult<Vec<u32>> {
        self.inner
            .encode_text(text)
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    /// Decodes token IDs into a strict UTF-8 string.
    fn decode(&self, ids: Vec<u32>) -> PyResult<String> {
        self.inner
            .decode_text(&ids)
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    /// Returns the vocabulary size.
    #[getter]
    fn vocab_size(&self) -> usize {
        self.inner.vocab_size()
    }
}

#[pymodule]
fn _neuratal_tokenizer(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyTokenizer>()?;
    Ok(())
}
