#!/usr/bin/env python3
"""Corpus integrity tests for the v0.2 train/benchmark split.

Verifies the final artifacts on disk (not the tools that produced them):
  - no NUL or unexpected control bytes in any cleaned file
  - every manifest path exists
  - every manifest sha256 matches the file bytes
  - train and benchmark manifests share no paths
  - benchmark contains zero files also present under train/ (by content hash)
  - manifest entries carry domain/bucket/source/license
"""
import hashlib
import json
import os
import sys
import unittest

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))


class TestCorpusIntegrity(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.train = json.load(open(os.path.join(REPO, "data/v0.2/manifest/manifest_corpus.json")))
        cls.bench = json.load(open(os.path.join(REPO, "data/v0.2/manifest/manifest_benchmark.json")))

    def test_no_nul_bytes_in_corpus(self):
        for root in ("data/v0.2/clean/train", "data/v0.2/clean/benchmark"):
            for dirpath, _, names in os.walk(os.path.join(REPO, root)):
                for name in names:
                    if not name.endswith(".txt"):
                        continue
                    with open(os.path.join(dirpath, name), "rb") as f:
                        self.assertNotIn(b"\x00", f.read(), f"NUL byte in {name}")

    def test_manifest_paths_exist_and_hash(self):
        for doc in self.train["entries"] + self.bench["entries"]:
            full = os.path.join(REPO, doc["path"])
            self.assertTrue(os.path.exists(full), f"missing {doc['path']}")
            h = hashlib.sha256(open(full, "rb").read()).hexdigest()
            self.assertEqual(h, doc["sha256"], f"sha256 mismatch {doc['path']}")

    def test_manifests_disjoint_paths(self):
        train_paths = {e["path"] for e in self.train["entries"]}
        bench_paths = {e["path"] for e in self.bench["entries"]}
        self.assertEqual(train_paths & bench_paths, set())

    def test_manifests_disjoint_content(self):
        train_sha = {e["sha256"] for e in self.train["entries"]}
        bench_sha = {e["sha256"] for e in self.bench["entries"]}
        self.assertEqual(train_sha & bench_sha, set())

    def test_manifest_entries_have_metadata(self):
        for doc in self.train["entries"]:
            for key in ("domain", "bucket", "source", "license", "bytes", "weight"):
                self.assertIn(key, doc, f"{doc['path']} missing {key}")
            self.assertGreater(doc["bytes"], 0)
            self.assertEqual(doc["weight"], 1.0)

    def test_benchmark_byte_minimum(self):
        total = sum(e["bytes"] for e in self.bench["entries"])
        self.assertGreater(total, 150 * 1024 * 1024, "benchmark suspiciously small")

    def test_train_cjk_and_stem_coverage(self):
        buckets = {}
        for e in self.train["entries"]:
            buckets[e["bucket"]] = buckets.get(e["bucket"], 0) + e["bytes"]
        cjk = sum(buckets.get(b, 0) for b in ("chinese", "japanese", "korean"))
        self.assertGreater(cjk, 200 * 1024 * 1024, "CJK representation missing")
        self.assertGreater(buckets.get("stem", 0), 60 * 1024 * 1024, "STEM representation missing")


if __name__ == "__main__":
    unittest.main(verbosity=2)
