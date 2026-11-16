#!/usr/bin/env python3
"""Apples-to-apples benchmark of external tokenizers on the 544-file development benchmark.

- Same manifest (data/v0.2/manifest/manifest_benchmark.json, sha256 002a8224...), same raw bytes.
- Raw text only; no chat templates, no normalization, no special-token processing.
  - tiktoken: allowed_special=set()
  - HF tokenizers lib: add_special_tokens=False
- Writes per-file JSONL and an aggregate summary JSON to the output directory.
- Records versions, revisions, hashes, and failures explicitly.
"""
import argparse, hashlib, json, sys, time, datetime, os

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--manifest', required=True)
    ap.add_argument('--outdir', required=True)
    args = ap.parse_args()

    manifest = json.load(open(args.manifest))
    files = [(e['path'], e.get('bucket')) for e in manifest['entries']]
    print(f'loaded {len(files)} files from {args.manifest}')

    import tiktoken
    from tokenizers import Tokenizer

    results = []

    # ---- o200k_base via tiktoken ----
    enc = tiktoken.get_encoding('o200k_base')
    per = []
    t0 = time.time()
    fail = []
    for path, bucket in files:
        b = open(path, 'rb').read()
        text = b.decode('utf-8')  # verified all-valid earlier
        ids = enc.encode(text, allowed_special=set())
        per.append({'path': path, 'bucket': bucket, 'bytes': len(b), 'tokens': len(ids)})
    o200k = {'name': 'o200k_base', 'lib': f'tiktoken {tiktoken.__version__}',
             'revision': 'tiktoken built-in rank file (downloaded by tiktoken at first use; version pinned)',
             'special_tokens': 'allowed_special=set() (special strings encoded as ordinary text)',
             'decode_tested': False, 'files': len(per),
             'tokens': sum(p['tokens'] for p in per), 'seconds': time.time() - t0, 'per_file': per,
             'failures': fail}

    # ---- Qwen2.5-7B-Instruct tokenizer.json (pinned) ----
    SNAP = os.environ.get('QWEN_SNAPSHOT', '<path-to-pinned-Qwen2.5-7B-Instruct/snapshot>')
    tj = SNAP + '/tokenizer.json'
    qrev = 'a09a35458c702b33eeacc393d103063234e8bc28'
    tj_hash = hashlib.sha256(open(tj, 'rb').read()).hexdigest()
    tok = Tokenizer.from_file(tj)
    per = []
    t0 = time.time()
    fail = []
    for path, bucket in files:
        b = open(path, 'rb').read()
        text = b.decode('utf-8')
        try:
            ids = tok.encode(text, add_special_tokens=False).ids
        except Exception as e:
            fail.append({'path': path, 'error': str(e)})
            continue
        per.append({'path': path, 'bucket': bucket, 'bytes': len(b), 'tokens': len(ids)})
    qwen = {'name': 'Qwen/Qwen2.5-7B-Instruct (tokenizer.json)', 'lib': 'tokenizers 0.23.3',
            'revision': qrev, 'tokenizer_json_sha256': tj_hash,
            'special_tokens': 'add_special_tokens=False (raw text only)',
            'vocab_size': tok.get_vocab_size(),
            'decode_tested': False, 'files': len(per),
            'tokens': sum(p['tokens'] for p in per), 'seconds': time.time() - t0, 'per_file': per,
            'failures': fail}

    import os
    os.makedirs(args.outdir, exist_ok=True)
    for res in (o200k, qwen):
        with open(f"{args.outdir}/{res['name'].split(' ')[0].split('/')[-1].replace('.', '_').replace('-7B-Instruct', '_instruct')}_perfile.json", 'w') as f:
            json.dump(res['per_file'], f)
    summary = {
        'benchmark': args.manifest,
        'benchmark_sha256': hashlib.sha256(open(args.manifest, 'rb').read()).hexdigest(),
        'date': datetime.datetime.now(datetime.timezone.utc).isoformat(),
        'o200k_base': {k: v for k, v in o200k.items() if k != 'per_file'},
        'qwen25_7b_instruct': {k: v for k, v in qwen.items() if k != 'per_file'},
    }
    with open(f'{args.outdir}/external_summary.json', 'w') as f:
        json.dump(summary, f, indent=2)
    print(json.dumps({k: {kk: vv for kk, vv in v.items()} for k, v in summary.items() if isinstance(v, dict)}, indent=2)[:2000])

if __name__ == '__main__':
    main()
