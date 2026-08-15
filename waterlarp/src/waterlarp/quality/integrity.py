"""Observable literal preservation without claiming full semantic preservation."""

from __future__ import annotations

import re
from collections import Counter
from dataclasses import dataclass

URL = re.compile(r"https?://[^\s)\]}>,]+")
NUMBER = re.compile(r"(?<![\w.])[+-]?(?:\d+(?:\.\d+)?|\.\d+)(?:[eE][+-]?\d+)?(?![\w.])")
CODE = re.compile(r"`{1,3}([^`]+)`{1,3}")
QUOTE = re.compile(r"(?:\"([^\"]+)\"|'([^']+)')")


@dataclass(frozen=True)
class IntegrityReport:
    source_counts: dict[str, int]
    preserved_counts: dict[str, int]
    recall: dict[str, float | None]


def extract_literals(text: str) -> dict[str, Counter[str]]:
    quotes = [next(group for group in match if group) for match in QUOTE.findall(text)]
    return {
        "numbers": Counter(NUMBER.findall(text)),
        "urls": Counter(URL.findall(text)),
        "code_spans": Counter(CODE.findall(text)),
        "quoted_literals": Counter(quotes),
    }


def literal_integrity(source: str, transformed: str) -> IntegrityReport:
    before = extract_literals(source)
    after = extract_literals(transformed)
    source_counts: dict[str, int] = {}
    preserved_counts: dict[str, int] = {}
    recall: dict[str, float | None] = {}
    for kind, values in before.items():
        total = values.total()
        preserved = sum(min(count, after[kind][value]) for value, count in values.items())
        source_counts[kind] = total
        preserved_counts[kind] = preserved
        recall[kind] = preserved / total if total else None
    return IntegrityReport(source_counts, preserved_counts, recall)
