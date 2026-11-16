//! Encode a corpus file with an `.ntok` model and write the token IDs as raw
//! little-endian u32 to stdout (or a file). Used to produce token streams for
//! the small-LM (BPB) pilot.
//!
//! Usage:
//!   target/release/examples/lm_encode <model.ntok> <input.txt> [output.bin]

use neuratal_tokenizer::load_ntok_file;
use std::io::{self, Write};
use std::path::PathBuf;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("usage: lm_encode <model.ntok> <input.txt> [output.bin]");
        std::process::exit(2);
    }
    let model_path = PathBuf::from(&args[1]);
    let input_path = PathBuf::from(&args[2]);
    let out_path = args.get(3).map(PathBuf::from);

    let tok = load_ntok_file(&model_path).expect("failed to load model");
    let bytes = std::fs::read(&input_path).expect("failed to read input");
    let ids = tok.encode_bytes(&bytes);

    let mut writer: Box<dyn Write> = match out_path {
        Some(p) => Box::new(std::fs::File::create(&p).expect("create output")),
        None => Box::new(io::stdout().lock()),
    };
    let mut buf = Vec::with_capacity(ids.len() * 4);
    for id in &ids {
        buf.extend_from_slice(&id.to_le_bytes());
    }
    writer.write_all(&buf).expect("write output");
    eprintln!(
        "{} bytes -> {} tokens ({:.4} bytes/token)",
        bytes.len(),
        ids.len(),
        bytes.len() as f64 / ids.len() as f64
    );
}
