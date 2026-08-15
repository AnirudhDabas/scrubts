#!/usr/bin/env python3
"""Reproduce the single Mega D-A controlled Unicode fixture."""

from __future__ import annotations

import hashlib
from pathlib import Path


HERE = Path(__file__).resolve().parent
VISIBLE_BYTES = b"This sentence is a controlled project fixture.\n"
INJECTED_BYTES = "\u200b".encode("utf-8")
BYTE_OFFSET = 4


def main() -> None:
    fixture = VISIBLE_BYTES[:BYTE_OFFSET] + INJECTED_BYTES + VISIBLE_BYTES[BYTE_OFFSET:]
    fixture_path = HERE / "controlled-u200b.txt"
    visible_path = HERE / "controlled-u200b.visible.txt"
    fixture_path.write_bytes(fixture)
    visible_path.write_bytes(VISIBLE_BYTES)
    print(
        "fixture=controlled-u200b.txt "
        f"bytes={len(fixture)} sha256={hashlib.sha256(fixture).hexdigest()}"
    )


if __name__ == "__main__":
    main()
