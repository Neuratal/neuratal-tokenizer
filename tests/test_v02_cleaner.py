#!/usr/bin/env python3
"""Unicode property tests for the v0.2 cleaner (data/v0.2/tools/clean_v02.py).

Verifies the invariants that v0.1 violated:
  - Devanagari matras, Arabic harakat, Thai vowels, Turkish İ, emoji ZWJ
    sequences and combining marks are never dropped or altered by the cleaner.
  - Leading/internal whitespace is always preserved; only line terminators change.
  - Binary detection rejects magic/NUL/UTF-8-invalid/control-heavy files and
    accepts extensionless text.
  - CRLF/CR are normalized to LF; blank lines survive.
"""
import sys
import os
import unittest

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(REPO, "data", "v0.2", "tools"))
import clean_v02  # noqa: E402


class TestUnicodeCleaning(unittest.TestCase):
    def test_devanagari_matra_preserved(self):
        text = "कि का कु\nहिन्दी लिपि में लिखा उदाहरण\n"
        out = clean_v02.clean_text(text, prose_policy=True)
        self.assertEqual(out, text)

    def test_turkish_capital_i_preserved(self):
        text = "İstanbul'da İ ve ı harfleri karışmamalı\nİzmir\n"
        out = clean_v02.clean_text(text, prose_policy=True)
        self.assertEqual(out, text)

    def test_zwj_emoji_sequence_preserved(self):
        text = "👨‍👩‍👧‍👦 aile 🚀 Türkçe\n🏴󠁧󠁢󠁥󠁮󠁧󠁿\n"
        out = clean_v02.clean_text(text, prose_policy=True)
        self.assertEqual(out, text)

    def test_arabic_harakat_preserved(self):
        text = "بِسْمِ اللَّهِ الرَّحْمَٰنِ\n"
        out = clean_v02.clean_text(text, prose_policy=True)
        self.assertEqual(out, text)

    def test_combining_marks_preserved(self):
        text = "cafe\u0301\n"  # NFD e + combining acute
        out = clean_v02.clean_text(text, prose_policy=True)
        self.assertEqual(out, text)

    def test_leading_indentation_and_tabs_preserved(self):
        text = "def outer():\n    if True:\n\t\tprint('x')\n        pass\n\n"
        out = clean_v02.clean_text(text, prose_policy=False)
        self.assertEqual(out, text)

    def test_crlf_cr_normalized_to_lf(self):
        text = "a\r\nb\rc\n"
        out = clean_v02.clean_text(text, prose_policy=False)
        self.assertEqual(out, "a\nb\nc\n")

    def test_blank_lines_preserved(self):
        text = "a\n\n\nb\n"
        out = clean_v02.clean_text(text, prose_policy=True)
        self.assertEqual(out, text)

    def test_prose_policy_strips_only_trailing_ws(self):
        text = "word  \n\tother\t\n"
        out = clean_v02.clean_text(text, prose_policy=True)
        self.assertEqual(out, "word\n\tother\n")

    def test_code_policy_keeps_trailing_ws(self):
        text = "x = 1  \n"
        out = clean_v02.clean_text(text, prose_policy=False)
        self.assertEqual(out, "x = 1  \n")


class TestBinaryDetection(unittest.TestCase):
    def check(self, raw, expect_binary):
        is_bin, reason = clean_v02.looks_binary(raw, "file.txt")
        self.assertEqual(is_bin, expect_binary, f"expected binary={expect_binary}, reason={reason}")

    def test_nul_bytes_rejected(self):
        self.check(b"hello\x00world", True)

    def test_magic_sqlite_rejected(self):
        self.check(b"SQLite format 3\x00rest", True)

    def test_magic_elf_rejected(self):
        self.check(b"\x7fELF\x02\x01\x01\x00payload", True)

    def test_magic_zip_rejected(self):
        self.check(b"PK\x03\x04rest", True)

    def test_control_ratio_rejected(self):
        bad = bytes([0x01, 0x02, 0x03] * 200 + [9, 10, 13] * 100)
        self.check(bad, True)

    def test_clean_text_accepted(self):
        self.check("hello world\nUnicode ✓ αβγ\n".encode("utf-8"), False)

    def test_extensionless_text_accepted(self):
        is_bin, reason = clean_v02.looks_binary("just text\n".encode("utf-8"), "LICENSE")
        self.assertFalse(is_bin)

    def test_invalid_utf8_rejected_at_decode_stage(self):
        text, enc = clean_v02.decode_bytes(b"\xff\xfe\x80abc", False)
        self.assertIsNone(text)

    def test_latin1_fallback_only_when_allowed(self):
        text, enc = clean_v02.decode_bytes(b"caf\xe9\n", True)
        self.assertEqual(text, "café\n")
        self.assertEqual(enc, "latin-1-fallback")


if __name__ == "__main__":
    unittest.main(verbosity=2)
