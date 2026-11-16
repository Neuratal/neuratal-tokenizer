## Summary

## Type of change
- [ ] Bug fix (reproduces and fixes a crash/wrong output)
- [ ] New feature (pretokenizer/algorithm/CLI/Python)
- [ ] Performance
- [ ] Documentation
- [ ] Tests / fixtures

## Checklist

- [ ] `cargo test --release` passes (136+ tests)
- [ ] `python3 tests/test_python_bindings.py` passes (if Python touched)
- [ ] No benchmark files, holdouts or corpora added (they are not distributed)
- [ ] No license text added/removed incorrectly; `LICENSE` untouched unless intended
- [ ] Unicode changes are covered by tests (`tests/unicode_regression_tests.rs`)
- [ ] Byte-exact round-trip preserved: `decode(encode(x)) == x` for your inputs
- [ ] Claims about performance/quality are measured, not asserted

## Details

Link related issues; describe measurement environment for performance claims.
