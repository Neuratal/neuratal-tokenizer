#!/usr/bin/env python3
"""
Integration example: feeding NTokenizer output into a PyTorch training pipeline.

Demonstrates how to use neuratal_tokenizer inside a real text pipeline:
  1. Load a trained .ntok model
  2. Tokenize a list of texts into id sequences
  3. Pad/truncate to a fixed context length with the <|pad|> special token
  4. Collate into a torch tensor batch and run a forward pass through a tiny
     embedding model as a smoke test

Requirements: a trained tokenizer (.ntok) and PyTorch. If torch is not installed,
the script prints an informative message and exits with code 2 (it does not
pretend to have verified PyTorch integration).

Usage:
    python3 examples/pytorch_training_example.py [path/to/tokenizer.ntok]
"""

import os
import sys
from typing import List

sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "python")))

from neuratal_tokenizer import Tokenizer, TokenizerError

try:
    import torch
    from torch import nn
except ImportError:
    print("PyTorch is not installed. Install it with: pip install torch")
    print("Tokenizer-side steps verified; the model forward pass was NOT run.")
    sys.exit(2)

CONTEXT_LENGTH = 64


def encode_samples(tok: Tokenizer, texts: List[str]):
    """Encode each text, pad/truncate to CONTEXT_LENGTH, and return tensors."""
    pad_id = tok.special_token_id("<|pad|>")
    if pad_id is None:
        raise TokenizerError("This tokenizer has no <|pad|> special token; pick one or pad with EOS.")

    all_ids = []
    for text in texts:
        ids = tok.encode(text)
        if len(ids) > CONTEXT_LENGTH:
            ids = ids[:CONTEXT_LENGTH]
        else:
            ids = ids + [pad_id] * (CONTEXT_LENGTH - len(ids))
        all_ids.append(ids)
    return torch.tensor(all_ids, dtype=torch.long)


def main() -> int:
    model_path = sys.argv[1] if len(sys.argv) > 1 else None
    if model_path:
        tok = Tokenizer.from_file(model_path)
        print(f"Loaded trained model: {model_path} (vocab_size={tok.vocab_size})")
    else:
        tok = Tokenizer.base()
        print(f"No model path given; using base byte tokenizer (vocab_size={tok.vocab_size})")

    batch_texts = [
        "The quick brown fox jumps over the lazy dog.",
        "Türkçe metin: doğal dil işleme ve derin öğrenme.",
        "Математические вычисления: 1 + 1 = 2.",
        "Mixed: code = def solve(x): return x * 42",
    ]

    input_ids = encode_samples(tok, batch_texts)
    print(f"input_ids: shape={tuple(input_ids.shape)} dtype={input_ids.dtype}")
    print(f"first row head: {input_ids[0][:12].tolist()}")

    vocab = tok.vocab_size
    embedding = nn.Embedding(vocab, 32)
    positions = torch.arange(CONTEXT_LENGTH).unsqueeze(0).expand(input_ids.shape[0], -1)

    embedded = embedding(input_ids) + embedding(positions)
    logits = nn.Linear(32, vocab)(embedded)
    loss = logits.log_softmax(dim=-1).mean()

    print(f"forward pass OK: embedding {tuple(embedding.weight.shape)} -> logits {tuple(logits.shape)}")
    print(f"loss (untrained, meaningless value): {loss.item():.3f}")
    print("Round-trip check:", tok.decode(input_ids[0].tolist())[:60])
    return 0


if __name__ == "__main__":
    sys.exit(main())
