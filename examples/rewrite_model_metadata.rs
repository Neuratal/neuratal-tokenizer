//! One-off repair: rewrite the embedded model_name of a v3 model and re-save it.
//! Used once for the v0.2 release to fix the temp-file name baked into metadata.
use neuratal_tokenizer::{load_ntok_file, save_ntok_file, ModelMetadata};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let input = &args[1];
    let output = &args[2];
    let new_name = &args[3];
    let mut tok = load_ntok_file(input).expect("load failed");
    let mut md: ModelMetadata = tok.metadata().expect("model has no embedded metadata").clone();
    md.model_name = new_name.to_string();
    tok.set_metadata(md);
    save_ntok_file(&tok, output).expect("save failed");
    println!("rewrote {} -> {} with model_name={:?}", input, output, new_name);
}
