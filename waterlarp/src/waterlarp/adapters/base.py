"""Minimal adapter boundary; model and tokenizer dependencies stay explicit."""

from __future__ import annotations

from abc import ABC, abstractmethod
from collections.abc import Iterable, Mapping
from dataclasses import dataclass
from typing import Any, Protocol

from waterlarp.authority import AuthorityRecord
from waterlarp.config import Comparator


class TokenizerLike(Protocol):
    def encode(self, text: str, **kwargs: Any) -> list[int]: ...
    def decode(self, token_ids: Iterable[int], **kwargs: Any) -> str: ...


@dataclass(frozen=True)
class DetectionScore:
    score: float
    statistic_name: str
    scored_units: int
    p_value: float | None
    raw_evidence: Mapping[str, Any]

    @property
    def scored_unit_count(self) -> int:
        return self.scored_units


@dataclass(frozen=True)
class CalibrationResult:
    threshold: float
    target_fpr: float
    negative_count: int
    false_positive_count: int
    empirical_calibration_exceedance: float
    comparator: Comparator
    status: str


class WatermarkAdapter(ABC):
    @abstractmethod
    def prepare_generation(self, model: Any, tokenizer: TokenizerLike, device: str) -> Any:
        """Return explicit generation integration state."""

    @abstractmethod
    def apply_generation(self, generation_state: Any, input_ids: Any, scores: Any) -> Any:
        """Apply the authoritative generation transform to model logits."""

    @abstractmethod
    def score_token_ids(self, token_ids: list[int], tokenizer: TokenizerLike) -> DetectionScore:
        """Score exact token IDs without retokenization ambiguity."""

    @abstractmethod
    def calibrate(self, negative_scores: Iterable[float], target_fpr: float) -> CalibrationResult:
        """Calibrate on negatives disjoint from test examples."""

    @abstractmethod
    def metadata(self) -> Mapping[str, Any]:
        """Return exact mechanism/configuration metadata."""

    @abstractmethod
    def authority(self) -> AuthorityRecord:
        """Return the authority record governing this adapter."""
