"""Named deterministic seeds with stable derivation across Python processes."""

from __future__ import annotations

import hashlib
import random


def derive_seed(master_seed: int, namespace: str, *, bits: int = 63) -> int:
    if master_seed < 0:
        raise ValueError("master_seed must be non-negative")
    if not namespace:
        raise ValueError("namespace must not be empty")
    if not 1 <= bits <= 256:
        raise ValueError("bits must be in 1..256")
    payload = f"waterlarp-rng-v1\0{master_seed}\0{namespace}".encode()
    value = int.from_bytes(hashlib.sha256(payload).digest(), "big")
    return value & ((1 << bits) - 1)


def python_rng(master_seed: int, namespace: str) -> random.Random:
    return random.Random(derive_seed(master_seed, namespace))


def benchmark_key(master_seed: int, mechanism: str) -> int:
    """Derive a non-zero research key without suggesting secrecy or deployment use."""
    return derive_seed(master_seed, f"benchmark-key/{mechanism}", bits=63) or 1
