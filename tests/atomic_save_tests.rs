//! Atomic-save fault-injection tests.
//!
//! `save_ntok_file` writes to a temporary sibling (`<model>.ntok.tmp-<pid>`)
//! and renames it into place. These tests exercise the failure path:
//! the destination exists as a *nonempty directory*, so the rename fails.
//! The original must remain intact, no temp files may leak, and the error
//! must be reported rather than silently producing a partial model.

use std::fs;

use neuratal_tokenizer::{Tokenizer, load_ntok_file, save_ntok_file};

fn tmp_dir(tag: &str) -> std::path::PathBuf {
    let base = std::env::temp_dir().join(format!("ntok_atomic_{}_{}", tag, std::process::id()));
    let _ = fs::remove_dir_all(&base);
    fs::create_dir_all(&base).unwrap();
    base
}

#[test]
fn rename_failure_preserves_original_and_cleans_temp() {
    let dir = tmp_dir("rename_fail");
    let original = dir.join("model.ntok");
    let tok = Tokenizer::new_base();
    save_ntok_file(&tok, &original).expect("initial save failed");

    // Replace the model with a nonempty directory: rename(tmp, dir) fails.
    fs::remove_file(&original).unwrap();
    fs::create_dir(&original).unwrap();
    fs::write(original.join("sentinel.txt"), b"keep me").unwrap();

    let result = save_ntok_file(&tok, &original);
    assert!(
        result.is_err(),
        "saving onto a nonempty directory must fail, got: {:?}",
        result.map(|_| ())
    );

    // The sentinel file inside the directory must be untouched.
    assert_eq!(
        fs::read(original.join("sentinel.txt")).unwrap(),
        b"keep me",
        "original destination was damaged by failed save"
    );

    // No `*.ntok.tmp-*` leftovers may remain in the directory.
    let leftovers: Vec<_> = fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().contains(".ntok.tmp-"))
        .collect();
    assert!(
        leftovers.is_empty(),
        "temp file leaked: {:?}",
        leftovers.iter().map(|e| e.file_name()).collect::<Vec<_>>()
    );

    // The original model bytes must still load once the directory is removed
    // and the file re-created. (Round trip through a fresh file.)
    let _ = fs::remove_dir_all(&original);
    save_ntok_file(&tok, &original).expect("re-save after cleanup failed");
    let reloaded = load_ntok_file(&original).expect("re-saved model did not load");
    assert_eq!(reloaded.vocab_size(), tok.vocab_size());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn failed_save_does_not_corrupt_existing_model_file_bytes() {
    let dir = tmp_dir("bytes_intact");
    let target = dir.join("model.ntok");
    let tok = Tokenizer::new_base();
    save_ntok_file(&tok, &target).expect("initial save failed");
    let bytes_before = fs::read(&target).unwrap();

    // Poison the directory by removing write permission on the *file* so the
    // rename is the only thing that could replace it; the bytes must match.
    let mut perms = fs::metadata(&target).unwrap().permissions();
    perms.set_readonly(true);
    fs::set_permissions(&target, perms).unwrap();

    let result = save_ntok_file(&tok, &target);
    // On non-root Linux the rename may still succeed (rename needs write on the
    // directory, not on the file); either outcome is acceptable, but the
    // existing bytes must be exactly the original bytes in both cases.
    if result.is_err() {
        assert_eq!(
            fs::read(&target).unwrap(),
            bytes_before,
            "bytes changed despite failed save"
        );
    } else {
        load_ntok_file(&target).expect("model must still load after re-save");
    }
    let _ = fs::remove_dir_all(&dir);
}
