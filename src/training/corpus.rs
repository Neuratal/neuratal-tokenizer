//! Streaming corpus ingestion for plain text (TXT) and JSON Lines (JSONL).
//!
//! Processes large files line-by-line or in streaming chunks without loading
//! the entire corpus into memory. Tracks and reports malformed records.

use serde_json::Value;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

/// Format of the training corpus file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CorpusFormat {
    Text,
    Jsonl,
}

impl CorpusFormat {
    /// Detects corpus format from file extension.
    pub fn from_path<P: AsRef<Path>>(path: P) -> Self {
        let p = path.as_ref();
        if let Some(ext) = p.extension().and_then(|s| s.to_str())
            && (ext.eq_ignore_ascii_case("jsonl") || ext.eq_ignore_ascii_case("json"))
        {
            return CorpusFormat::Jsonl;
        }
        CorpusFormat::Text
    }
}

/// A successfully ingested record from a corpus file.
#[derive(Debug, Clone)]
pub struct CorpusRecord {
    pub text: String,
    pub language: Option<String>,
    pub domain: Option<String>,
    pub file_path: String,
    pub line_number: usize,
}

/// A malformed or unparseable record encountered during ingestion.
#[derive(Debug, Clone)]
pub struct CorpusRecordError {
    pub file_path: String,
    pub line_number: usize,
    pub reason: String,
}

/// Streaming reader that ingests a corpus file line-by-line.
pub struct StreamingCorpusReader {
    path: PathBuf,
    format: CorpusFormat,
    text_field: String,
    reader: BufReader<File>,
    line_number: usize,
}

impl StreamingCorpusReader {
    /// Opens a corpus file for streaming ingestion.
    pub fn open<P: AsRef<Path>>(
        path: P,
        format: CorpusFormat,
        text_field: Option<String>,
    ) -> Result<Self, std::io::Error> {
        let path_buf = path.as_ref().to_path_buf();
        let file = File::open(&path_buf)?;
        let reader = BufReader::new(file);

        Ok(Self {
            path: path_buf,
            format,
            text_field: text_field.unwrap_or_else(|| "text".to_string()),
            reader,
            line_number: 0,
        })
    }

    /// Reads the next record from the stream.
    ///
    /// Returns:
    /// - `Some(Ok(record))` on a valid record.
    /// - `Some(Err(error))` on a malformed record (reported, not silently dropped).
    /// - `None` when EOF is reached.
    pub fn next_record(&mut self) -> Option<Result<CorpusRecord, CorpusRecordError>> {
        let mut line = String::new();
        loop {
            line.clear();
            self.line_number += 1;
            match self.reader.read_line(&mut line) {
                Ok(0) => return None, // EOF
                Ok(_) => {
                    // v0.2: the record keeps the line VERBATIM, including the
                    // line terminator (\n or \r\n as stored on disk; v0.2 corpus
                    // files are already LF-normalized by the cleaner). Leading
                    // indentation and tabs are preserved exactly. Blank lines
                    // are emitted as one-line "\n" records so BPE can learn
                    // newline/paragraph merges. (The v0.1 reader trimmed every
                    // line with trim(), destroying code indentation.)
                    let raw_line = line.as_str();

                    let file_path_str = self.path.to_string_lossy().to_string();

                    match self.format {
                        CorpusFormat::Text => {
                            return Some(Ok(CorpusRecord {
                                text: raw_line.to_string(),
                                language: None,
                                domain: None,
                                file_path: file_path_str,
                                line_number: self.line_number,
                            }));
                        }
                        CorpusFormat::Jsonl => {
                            if raw_line.trim().is_empty() {
                                // Blank JSONL lines are formatting, not records:
                                // skip without reporting. (TXT keeps them.)
                                continue;
                            }
                            match serde_json::from_str::<Value>(raw_line) {
                            Ok(json) => {
                                if let Some(obj) = json.as_object() {
                                    if let Some(val) = obj.get(&self.text_field) {
                                        if let Some(text_str) = val.as_str() {
                                            let lang = obj
                                                .get("language")
                                                .or_else(|| obj.get("lang"))
                                                .and_then(|v| v.as_str())
                                                .map(|s| s.to_string());
                                            let domain = obj
                                                .get("domain")
                                                .and_then(|v| v.as_str())
                                                .map(|s| s.to_string());

                                            return Some(Ok(CorpusRecord {
                                                text: text_str.to_string(),
                                                language: lang,
                                                domain,
                                                file_path: file_path_str,
                                                line_number: self.line_number,
                                            }));
                                        } else {
                                            return Some(Err(CorpusRecordError {
                                                file_path: file_path_str,
                                                line_number: self.line_number,
                                                reason: format!(
                                                    "Field '{}' is not a string",
                                                    self.text_field
                                                ),
                                            }));
                                        }
                                    } else {
                                        return Some(Err(CorpusRecordError {
                                            file_path: file_path_str,
                                            line_number: self.line_number,
                                            reason: format!(
                                                "Missing required text field '{}'",
                                                self.text_field
                                            ),
                                        }));
                                    }
                                } else {
                                    return Some(Err(CorpusRecordError {
                                        file_path: file_path_str,
                                        line_number: self.line_number,
                                        reason: "Root JSON is not an object".to_string(),
                                    }));
                                }
                            }
                            Err(e) => {
                                return Some(Err(CorpusRecordError {
                                    file_path: file_path_str,
                                    line_number: self.line_number,
                                    reason: format!("JSON parse error: {}", e),
                                }));
                            }
                            }
                        }
                    }
                }
                Err(e) => {
                    return Some(Err(CorpusRecordError {
                        file_path: self.path.to_string_lossy().to_string(),
                        line_number: self.line_number,
                        reason: format!("I/O read error: {}", e),
                    }));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_streaming_txt_reader() {
        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("test_sample.txt");
        {
            let mut f = File::create(&file_path).unwrap();
            writeln!(f, "First line of text").unwrap();
            writeln!(f).unwrap(); // Empty line
            writeln!(f, "Second line of text").unwrap();
        }

        let mut reader = StreamingCorpusReader::open(&file_path, CorpusFormat::Text, None).unwrap();
        // v0.2: TXT records keep the line terminator; blank lines are "\n" records.
        let r1 = reader.next_record().unwrap().unwrap();
        assert_eq!(r1.text, "First line of text\n");
        assert_eq!(r1.line_number, 1);

        let r_blank = reader.next_record().unwrap().unwrap();
        assert_eq!(r_blank.text, "\n");
        assert_eq!(r_blank.line_number, 2);

        let r2 = reader.next_record().unwrap().unwrap();
        assert_eq!(r2.text, "Second line of text\n");
        assert_eq!(r2.line_number, 3);

        assert!(reader.next_record().is_none());
        let _ = std::fs::remove_file(file_path);
    }

    #[test]
    fn test_streaming_jsonl_reader_with_error_reporting() {
        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("test_sample.jsonl");
        {
            let mut f = File::create(&file_path).unwrap();
            writeln!(f, r#"{{"text": "Valid line", "language": "en"}}"#).unwrap();
            writeln!(f, r#"{{"invalid_json": "missing text"}}"#).unwrap();
            writeln!(f, r#"NOT JSON AT ALL"#).unwrap();
            writeln!(f, r#"{{"text": "Another valid line", "domain": "math"}}"#).unwrap();
        }

        let mut reader =
            StreamingCorpusReader::open(&file_path, CorpusFormat::Jsonl, None).unwrap();

        // 1. Valid record
        let r1 = reader.next_record().unwrap().unwrap();
        assert_eq!(r1.text, "Valid line");
        assert_eq!(r1.language.as_deref(), Some("en"));

        // 2. Missing text field error
        let err1 = reader.next_record().unwrap().unwrap_err();
        assert_eq!(err1.line_number, 2);
        assert!(err1.reason.contains("Missing required text field"));

        // 3. Malformed JSON syntax error
        let err2 = reader.next_record().unwrap().unwrap_err();
        assert_eq!(err2.line_number, 3);
        assert!(err2.reason.contains("JSON parse error"));

        // 4. Valid record
        let r2 = reader.next_record().unwrap().unwrap();
        assert_eq!(r2.text, "Another valid line");
        assert_eq!(r2.domain.as_deref(), Some("math"));

        assert!(reader.next_record().is_none());
        let _ = std::fs::remove_file(file_path);
    }
}
