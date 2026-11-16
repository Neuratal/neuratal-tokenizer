#!/usr/bin/env python3
"""Aggregate the apples-to-apples comparison on the 544-file development benchmark.

Reads the per-file JSONL written by the native harness (examples/benchmark_ntok
--per-file-json) and by scripts/measure_external.py, then emits bucket-level B/T,
overall B/T, equal-category B/T (arithmetic mean of the 11 bucket B/T values),
token reduction vs o200k_base and vs Qwen2.5-7B-Instruct.

No benchmark text is included in the output; only paths, byte counts, token counts.
Usage: python3 scripts/aggregate_comparison.py --results results --manifest ../../data/v0.2/manifest/manifest_benchmark.json
"""
import argparse, csv, json, collections, os, sys

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--results', default='results')
    ap.add_argument('--manifest', default='../../data/v0.2/manifest/manifest_benchmark.json')
    args = ap.parse_args()
    buckets = ['arabic','chinese','english','hindi','japanese','korean','other_languages','programming','spanish','stem','turkish']

    mods = {
        'v0.1':          'public-benchmark-perfile-v0.1.json',
        'v0.2':          'public-benchmark-perfile-v0.2.json',
        'v0.3-original': 'public-benchmark-perfile-v03orig.json',
        'v0.3-polished': 'public-benchmark-perfile-v0.3-polished.json',
        'v0.4-fail-cjk8':'public-benchmark-perfile-v0.4-fail.json',
        'v0.4-fail-han4':'public-benchmark-perfile-v0.4-fail-han4.json',
        'o200k':         'o200k_base_perfile.json',
        'qwen2.5-7b-instruct': 'Qwen2_5_instruct_perfile.json',
    }
    vocab = {'v0.1':262144,'v0.2':262144,'v0.3-original':262144,'v0.3-polished':262144,'v0.4-fail-cjk8':262144,'v0.4-fail-han4':262144,'o200k':200019,'qwen2.5-7b-instruct':151665}

    data = {}
    for k, f in mods.items():
        per = json.load(open(os.path.join(args.results, f)))
        agg = collections.defaultdict(lambda: [0, 0])
        for e in per:
            a = agg[e['bucket']]; a[0] += e['bytes']; a[1] += e['tokens']
        data[k] = {'total_bytes': sum(v[0] for v in agg.values()),
                   'total_tokens': sum(v[1] for v in agg.values()),
                   **{b: tuple(agg[b]) for b in buckets}}
        data[k]['overall_bt'] = data[k]['total_bytes'] / data[k]['total_tokens']
        data[k]['equal_bt'] = sum(data[k][b][0]/data[k][b][1] for b in buckets) / len(buckets)

    o200k_tok = data['o200k']['total_tokens']
    qwen_tok = data['qwen2.5-7b-instruct']['total_tokens']
    order = list(mods)
    with open(os.path.join(args.results, '..', 'STANDARDIZED_RESULTS.csv'), 'w', newline='') as f:
        w = csv.writer(f)
        w.writerow(['model','vocabulary','tokens','overall_bytes_per_token','equal_category_bytes_per_token','token_reduction_vs_o200k_pct','token_reduction_vs_qwen_pct'])
        for k in order:
            o = data[k]
            w.writerow([k, vocab[k], o['total_tokens'], f'{o["overall_bt"]:.4f}', f'{o["equal_bt"]:.4f}',
                        f'{100*(o200k_tok-o["total_tokens"])/o200k_tok:+.2f}', f'{100*(qwen_tok-o["total_tokens"])/qwen_tok:+.2f}'])
    with open(os.path.join(args.results, '..', 'CATEGORY_RESULTS.csv'), 'w', newline='') as f:
        w = csv.writer(f)
        w.writerow(['bucket','bytes','metric'] + order)
        for b in buckets:
            w.writerow([b, data['v0.1'][b][0], 'B/T'] + [f'{data[k][b][0]/data[k][b][1]:.4f}' for k in order])
            w.writerow([b, data['v0.1'][b][0], 'tokens'] + [data[k][b][1] for k in order])
    for k in order:
        print(f'{k:22} tokens={data[k]["total_tokens"]:>12}  B/T={data[k]["overall_bt"]:.4f}  equal={data[k]["equal_bt"]:.4f}')

if __name__ == '__main__':
    main()
