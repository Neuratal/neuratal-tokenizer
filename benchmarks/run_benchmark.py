#!/usr/bin/env python3
r"""
NTokenizer v0.2 held-out benchmark runner.

Measures, over the FULL held-out benchmark tree (data/v0.2/clean/benchmark),
per model and per category (bucket derived from file name):

  bytes/token   (higher = better compression)
  tokens/byte
  tokens/word   (word = \S+ run, Unicode whitespace)
  avg bytes/token (== bytes/token; duplicated as "avg_token_len_bytes")
  total bytes / total tokens
  encode time (s, MB/s)
  decode exactness (decode(encode(bytes)) == bytes) — reported per model;
  for NTokenizer byte-level models this MUST be True; external tokenizers
  are reported as measured (WordPiece is not lossless).

Models compared:
  NTokenizer v0.1 baseline (output/ntokenizer-256k.ntok) — for regression only
  NTokenizer v0.2 exports (output/v0.2/*.ntok)
  tiktoken: gpt2 (r50k_base), cl100k_base, o200k_base
  HuggingFace tokenizers: Qwen2.5 (tokenizer.json), bert-base-multilingual-cased (tokenizer.json)

Usage:
  python3 benchmarks/run_benchmark.py --benchmark data/v0.2/clean/benchmark \
      --reports reports/v0.2 --only gpt2,o200k_base   (optional name filter)
"""
import argparse
import json
import os
import re
import sys
import time

WS_RE = re.compile(r"\S+", re.UNICODE)
_REPO_ROOT_FOR_BINDINGS = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "python")
if _REPO_ROOT_FOR_BINDINGS not in sys.path:
    sys.path.insert(0, _REPO_ROOT_FOR_BINDINGS)

DOMAIN_BUCKET = {
    "simplewiki": "english", "enwikinews": "english", "enwikibooks": "english", "gutenberg_en": "english",
    "trwiki": "turkish", "trwikisource": "turkish", "trwiktionary": "turkish",
    "arwikisource": "arabic", "hiwiki": "hindi", "eswikisource": "spanish",
    "zhwiktionary": "chinese", "gutenberg_cjk_zh": "chinese",
    "jawikisource": "japanese", "jawiktionary": "japanese", "gutenberg_cjk_ja": "japanese",
    "kowikisource": "korean", "kowiktionary": "korean",
    "cpython_code": "programming", "cpython_docs": "programming", "rust_src": "programming",
    "go_src": "programming", "typescript_src": "programming",
    "gutenberg_science": "stem", "gutenberg_science2": "stem", "arxiv": "stem",
}
TATOEBA_BUCKETS = {
    "tatoeba_eng": "english", "tatoeba_spa": "spanish", "tatoeba_tur": "turkish", "tatoeba_ara": "arabic",
    "tatoeba_hin": "hindi", "tatoeba_cmn": "chinese", "tatoeba_jpn": "japanese", "tatoeba_kor": "korean",
}
KNOWN_DOMAINS = sorted(set(list(DOMAIN_BUCKET) + list(TATOEBA_BUCKETS)), key=len, reverse=True)


def bucket_of(filename):
    for d in KNOWN_DOMAINS:
        if filename.startswith(d + "_"):
            return DOMAIN_BUCKET.get(d, TATOEBA_BUCKETS.get(d, "other_languages"))
    return "other_languages"


def domain_of(filename):
    for d in KNOWN_DOMAINS:
        if filename.startswith(d + "_"):
            return d
    return filename.split("_", 1)[0]


def iter_benchmark_files(root):
    for dirpath, _, names in os.walk(root):
        for name in names:
            if name.endswith(".txt"):
                yield os.path.join(dirpath, name)


class ModelAdapter:
    name = "base"
    def encode(self, text):
        raise NotImplementedError
    def decode(self, ids):
        raise NotImplementedError
    def n_vocab(self):
        return None


class NTokenizerAdapter(ModelAdapter):
    def __init__(self, path, name):
        from neuratal_tokenizer import Tokenizer  # ctypes bindings (python/neuratal_tokenizer)
        self._tok = Tokenizer.from_file(path)
        self.name = name
        self.path = path
    def encode(self, text):
        return self._tok.encode(text)
    def decode(self, ids):
        return self._tok.decode(ids)
    def n_vocab(self):
        try:
            return self._tok.vocab_size()
        except Exception:
            return None


class TiktokenAdapter(ModelAdapter):
    def __init__(self, encoding_name, name):
        import tiktoken
        self._enc = tiktoken.get_encoding(encoding_name)
        self.name = name
    def encode(self, text):
        return self._enc.encode(text)
    def decode(self, ids):
        return self._enc.decode(ids)
    def n_vocab(self):
        return self._enc.n_vocab


class HFTokenizerAdapter(ModelAdapter):
    def __init__(self, path, name):
        from tokenizers import Tokenizer
        self._tok = Tokenizer.from_file(path)
        self.name = name
    def encode(self, text):
        return self._tok.encode(text).ids
    def decode(self, ids):
        return self._tok.decode(ids)
    def n_vocab(self):
        return self._tok.get_vocab_size()


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--benchmark", required=True)
    ap.add_argument("--reports", required=True)
    ap.add_argument("--ntok-dir", default="output", help="directory for NTokenizer .ntok models")
    ap.add_argument("--v02-dir", default="output/v0.2")
    ap.add_argument("--repo-root", default=os.getcwd())
    ap.add_argument("--only", default="")
    args = ap.parse_args()

    files = sorted(iter_benchmark_files(args.benchmark))
    total_bytes = sum(os.path.getsize(f) for f in files)
    print(f"benchmark: {len(files)} files, {total_bytes/1e6:.1f} MB", file=sys.stderr)

    models = []
    # NTokenizer v0.1 baseline
    v01 = os.path.join(args.repo_root, args.ntok_dir, "ntokenizer-256k.ntok")
    if os.path.exists(v01):
        models.append(NTokenizerAdapter(v01, "ntokenizer-0.1-256k"))
    # NTokenizer v0.2 family
    if os.path.isdir(os.path.join(args.repo_root, args.v02_dir)):
        for name in sorted(os.listdir(os.path.join(args.repo_root, args.v02_dir))):
            if name.endswith(".ntok"):
                models.append(NTokenizerAdapter(os.path.join(args.repo_root, args.v02_dir, name), "ntokenizer-0.2-" + name[:-5]))
    # external tokenizers
    ext = os.path.join(args.repo_root, "benchmarks", "external")
    models.append(TiktokenAdapter("gpt2", "gpt2-r50k"))
    models.append(TiktokenAdapter("cl100k_base", "cl100k_base"))
    models.append(TiktokenAdapter("o200k_base", "o200k_base"))
    models.append(HFTokenizerAdapter(os.path.join(ext, "qwen2", "tokenizer.json"), "qwen2.5"))
    models.append(HFTokenizerAdapter(os.path.join(ext, "bert-multilingual-cased", "tokenizer.json"), "bert-multilingual-cased"))

    only = {s.strip() for s in args.only.split(",") if s.strip()}
    models = [m for m in models if not only or m.name in only]

    # Pre-load all texts once (memory: ~160MB, fine)
    texts = []
    for f in files:
        with open(f, "rb") as fh:
            raw = fh.read()
        texts.append(raw.decode("utf-8"))
    file_buckets = [bucket_of(os.path.basename(f)) for f in files]
    file_domains = [domain_of(os.path.basename(f)) for f in files]
    file_bytes = [len(t.encode("utf-8")) for t in texts]
    file_words = [len(WS_RE.findall(t)) for t in texts]

    results = []
    for model in models:
        print(f"=== {model.name} ===", file=sys.stderr)
        per_bucket = {}
        tot_tokens = 0
        tot_bytes = 0
        tot_words = 0
        enc_time = 0.0
        decode_ok = 0
        decode_fail = 0
        t0 = time.time()
        for i, text in enumerate(texts):
            s = time.time()
            ids = model.encode(text)
            enc_time += time.time() - s
            b = file_bytes[i]
            w = file_words[i]
            bucket = file_buckets[i]
            agg = per_bucket.setdefault(bucket, {"bytes": 0, "tokens": 0, "words": 0, "files": 0})
            agg["bytes"] += b
            agg["tokens"] += len(ids)
            agg["words"] += w
            agg["files"] += 1
            tot_tokens += len(ids)
            tot_bytes += b
            tot_words += w
        wall = time.time() - t0
        # decode check (sampled for speed: every file, but only once per model)
        for i, text in enumerate(texts):
            ids = model.encode(text)
            dec = model.decode(ids)
            same = (dec == texts[i])
            if same:
                decode_ok += 1
            else:
                decode_fail += 1
        n_vocab = model.n_vocab()
        buckets_out = {}
        for bucket, agg in sorted(per_bucket.items()):
            buckets_out[bucket] = {
                "files": agg["files"],
                "bytes": agg["bytes"],
                "tokens": agg["tokens"],
                "words": agg["words"],
                "bytes_per_token": round(agg["bytes"] / max(1, agg["tokens"]), 4),
                "tokens_per_byte": round(agg["tokens"] / max(1, agg["bytes"]), 6),
                "tokens_per_word": round(agg["tokens"] / max(1, agg["words"]), 4),
            }
        results.append({
            "model": model.name,
            "n_vocab": n_vocab,
            "files": len(files),
            "total_bytes": tot_bytes,
            "total_tokens": tot_tokens,
            "total_words": tot_words,
            "bytes_per_token": round(tot_bytes / max(1, tot_tokens), 4),
            "tokens_per_byte": round(tot_tokens / max(1, tot_bytes), 6),
            "tokens_per_word": round(tot_tokens / max(1, tot_words), 4),
            "avg_token_len_bytes": round(tot_bytes / max(1, tot_tokens), 4),
            "encode_seconds": round(enc_time, 3),
            "encode_mb_per_s": round(tot_bytes / max(1e-9, enc_time) / 1e6, 2),
            "decode_exact_files": decode_ok,
            "decode_mismatch_files": decode_fail,
            "by_bucket": buckets_out,
        })
        print(f"  bytes/token={results[-1]['bytes_per_token']}  enc={results[-1]['encode_mb_per_s']}MB/s  decode_ok={decode_ok}/{len(files)}", file=sys.stderr)

    os.makedirs(args.reports, exist_ok=True)
    raw_out = os.path.join(args.reports, "benchmark_results.json")
    with open(raw_out, "w", encoding="utf-8") as f:
        json.dump({
            "benchmark": os.path.relpath(args.benchmark, args.repo_root),
            "files": len(files),
            "total_bytes": total_bytes,
            "results": results,
        }, f, indent=1, ensure_ascii=False)
    print(f"wrote {raw_out}", file=sys.stderr)


if __name__ == "__main__":
    main()
