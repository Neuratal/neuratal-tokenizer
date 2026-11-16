#![no_main]

use libfuzzer_sys::fuzz_target;

// The deserializer must never panic on arbitrary input files; it returns Ok or Err.
fuzz_target!(|data: &[u8]| {
    let path = std::env::temp_dir().join(format!(
        "ntok_fuzz_load_{}_{}.ntok",
        std::process::id(),
        data.len()
    ));
    if std::fs::write(&path, data).is_err() {
        return;
    }
    let _ = neuratal_tokenizer::load_ntok_file(&path);
    let _ = std::fs::remove_file(&path);
});
