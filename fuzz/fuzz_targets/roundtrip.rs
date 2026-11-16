#![no_main]

use libfuzzer_sys::fuzz_target;
use neuratal_tokenizer::Tokenizer;

// Core invariant: decode_bytes(encode_bytes(x)) == x for arbitrary bytes.
fuzz_target!(|data: &[u8]| {
    let tok = Tokenizer::new_base();
    let ids = tok.encode_bytes(data);
    let out = tok
        .decode_bytes(&ids)
        .expect("decode_bytes must not fail on ids produced by encode_bytes");
    assert_eq!(out, data);
});
