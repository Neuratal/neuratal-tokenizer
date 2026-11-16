#!/usr/bin/env bash
# Example: train a small Neuratal Tokenizer model on a toy corpus and validate it.
#
# Usage:
#   examples/train_example.sh [corpus_file]
#
# If no corpus is provided, a small synthetic demo corpus is generated in a temp dir.
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

CLI="./target/release/ntokenizer"

if [ ! -x "${CLI}" ]; then
    echo "Release CLI not found at ${CLI}. Build it first:"
    echo "  cargo build --release"
    exit 1
fi

WORKDIR="$(mktemp -d -t ntok_train_example.XXXXXX)"
trap 'rm -rf "${WORKDIR}"' EXIT

CORPUS="${1:-${WORKDIR}/demo_corpus.txt}"
MODEL="${WORKDIR}/demo_tokenizer.ntok"

if [ $# -lt 1 ]; then
    python3 - <<'PY' > "${CORPUS}"
lines = []
samples = [
    "The quick brown fox jumps over the lazy dog.",
    "Autoregressive language models predict the next token in a sequence.",
    "Yapay zeka modelleri çok dilli veri kümeleriyle eğitilir.",
    "Алгоритмы машинного обучения обрабатывают текст.",
    "تعتمد النماذج اللغوية على خوارزميات دقيقة.",
    "深度学习自然语言处理模型高效地处理多语言文本。",
    "def loss(y_true, y_pred):\n    return ((y_true - y_pred) ** 2).mean()",
    "Numbers: 1234567890, 3.14159, 6.02214076e+23, 0xDEADBEEF.",
    "Emoji sequences with ZWJ: 👨‍👩‍👧‍👦 🚀 🇹🇷 1️⃣",
    "SELECT id, COUNT(*) FROM events GROUP BY id;",
]
for i in range(600):
    lines.append(samples[i % len(samples)])
print("\n".join(lines))
PY
fi

echo "== Corpus: ${CORPUS}"
echo "== Training (target vocab size 512 for the demo)..."
"${CLI}" train \
    --input "${CORPUS}" \
    --vocab-size 512 \
    --min-freq 2 \
    --pretokenizer conservative \
    --seed 42 \
    --output "${MODEL}"

echo
echo "== Inspecting the trained model"
"${CLI}" inspect --model "${MODEL}"

echo
echo "== Validating the trained model (with multilingual round-trip probes)"
"${CLI}" validate --model "${MODEL}" --roundtrip

echo
echo "== Note: the demo model is tiny (vocab 512) and is written to a temp directory."
echo "== Produce your own model by passing a corpus path: examples/train_example.sh data/corpus.txt"
