"""Deterministic benign operations; source text is never changed in place."""

from __future__ import annotations

import re
import unicodedata


def unicode_nfc(text: str) -> str:
    return unicodedata.normalize("NFC", text)


def normalize_whitespace(text: str) -> str:
    lines = (re.sub(r"[\t ]+", " ", line).rstrip() for line in text.splitlines())
    return "\n".join(lines).strip()


def normalize_line_endings(text: str) -> str:
    return text.replace("\r\n", "\n").replace("\r", "\n")


def bullet_format(text: str) -> str:
    sentences = [part.strip() for part in re.split(r"(?<=[.!?])\s+", text) if part.strip()]
    return "\n".join(f"- {sentence}" for sentence in sentences)


def delete_sentence(text: str, index: int) -> str:
    sentences = [part for part in re.split(r"(?<=[.!?])\s+", text) if part]
    if not 0 <= index < len(sentences):
        raise IndexError("sentence index out of range")
    return " ".join(sentence for i, sentence in enumerate(sentences) if i != index)
