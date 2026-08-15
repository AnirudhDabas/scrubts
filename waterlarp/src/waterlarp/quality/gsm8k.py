"""GSM8K final numerical answer preservation."""

from __future__ import annotations

import re
from decimal import Decimal, InvalidOperation

ANSWER = re.compile(r"####\s*([+-]?[\d,]+(?:\.\d+)?)")
FALLBACK = re.compile(r"[+-]?[\d,]+(?:\.\d+)?")


def final_number(text: str) -> Decimal | None:
    matches = ANSWER.findall(text) or FALLBACK.findall(text)
    if not matches:
        return None
    try:
        return Decimal(matches[-1].replace(",", ""))
    except InvalidOperation:
        return None


def answer_preserved(source: str, transformed: str) -> bool | None:
    before, after = final_number(source), final_number(transformed)
    return None if before is None or after is None else before == after
