"""Pinned KGW adapter over the authors' extended reference implementation."""

from __future__ import annotations

import importlib.util
import math
import sys
from collections.abc import Iterable, Mapping
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Any

from waterlarp.adapters.base import (
    CalibrationResult,
    DetectionScore,
    TokenizerLike,
    WatermarkAdapter,
)
from waterlarp.authority import KGW_AUTHORITY, AuthorityRecord
from waterlarp.calibration.thresholds import calibrate_threshold
from waterlarp.config import Comparator

KGW_REFERENCE_COMMIT = "82922516930c02f8aa322765defdb5863d07a00e"


@dataclass(frozen=True)
class KgwConfig:
    gamma: float = 0.25
    delta: float = 2.0
    context_width: int = 4
    prf_type: str = "anchored_minhash_prf"
    self_salt: bool = True
    base_key: int = 0
    ignore_repeated_ngrams: bool = True
    device: str = "cpu"
    rng: str = "torch.Generator"

    @property
    def seeding_scheme(self) -> str:
        return f"ff-{self.prf_type}-{self.context_width}-{self.self_salt}-{self.base_key}"

    def validate(self) -> None:
        if not 0 < self.gamma < 1 or self.delta <= 0 or self.context_width <= 0:
            raise ValueError("invalid KGW gamma, delta, or context width")
        if self.base_key == 15485863:
            raise ValueError("the published KGW demo key is forbidden for WaterLARP benchmark runs")
        if not self.ignore_repeated_ngrams:
            raise ValueError("the authoritative recommended detector ignores repeated n-grams")


def _load_reference(checkout: Path) -> Any:
    expected = checkout / "extended_watermark_processor.py"
    if not expected.is_file() or not (checkout / "alternative_prf_schemes.py").is_file():
        raise FileNotFoundError("pinned KGW checkout lacks required reference files")
    sys.path.insert(0, str(checkout))
    try:
        spec = importlib.util.spec_from_file_location("waterlarp_kgw_reference", expected)
        if spec is None or spec.loader is None:
            raise ImportError("could not load KGW reference module")
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        return module
    finally:
        sys.path.remove(str(checkout))


class KgwAdapter(WatermarkAdapter):
    def __init__(self, config: KgwConfig, reference_checkout: Path) -> None:
        config.validate()
        self.config = config
        self.reference_checkout = reference_checkout
        self.reference = _load_reference(reference_checkout)
        self._processor: Any = None
        self._detector: Any = None

    def prepare_generation(self, model: Any, tokenizer: TokenizerLike, device: str) -> Any:
        if device != self.config.device:
            raise ValueError("generation device must match frozen KGW RNG provenance")
        vocab = list(tokenizer.get_vocab().values())  # type: ignore[attr-defined]
        self._processor = self.reference.WatermarkLogitsProcessor(
            vocab=vocab,
            gamma=self.config.gamma,
            delta=self.config.delta,
            seeding_scheme=self.config.seeding_scheme,
        )
        return self._processor

    def apply_generation(self, generation_state: Any, input_ids: Any, scores: Any) -> Any:
        return generation_state(input_ids, scores)

    def _make_detector(self, tokenizer: TokenizerLike) -> Any:
        return self.reference.WatermarkDetector(
            vocab=list(tokenizer.get_vocab().values()),  # type: ignore[attr-defined]
            gamma=self.config.gamma,
            seeding_scheme=self.config.seeding_scheme,
            device=self.config.device,
            tokenizer=tokenizer,
            z_threshold=math.inf,
            normalizers=[],
            ignore_repeated_ngrams=self.config.ignore_repeated_ngrams,
        )

    def score_token_ids(self, token_ids: list[int], tokenizer: TokenizerLike) -> DetectionScore:
        if len(token_ids) <= self.config.context_width:
            return DetectionScore(
                0.0,
                "kgw_unique_ngram_z",
                0,
                1.0,
                {
                    "num_tokens_scored": 0,
                    "green_fraction": None,
                    "green_token_count": 0,
                    "z_score": 0.0,
                    "p_value": 1.0,
                    "gamma": self.config.gamma,
                    "repeated_ngram_policy": "unique_self_salted_ngrams",
                    "seeding_scheme": self.config.seeding_scheme,
                    "score_status": "UNSUPPORTED_NO_VALID_NGRAMS",
                },
            )
        detector = self._make_detector(tokenizer)
        import torch

        # Exact token IDs are the research primitive. Calling the reference
        # sequence scorer avoids decode/re-tokenize drift.
        raw = detector._score_sequence(torch.tensor(token_ids, device=self.config.device))
        return DetectionScore(
            float(raw["z_score"]),
            "kgw_unique_ngram_z",
            int(raw["num_tokens_scored"]),
            float(raw["p_value"]),
            {
                **dict(raw),
                "gamma": self.config.gamma,
                "repeated_ngram_policy": "unique_self_salted_ngrams",
                "seeding_scheme": self.config.seeding_scheme,
            },
        )

    def calibrate(self, negative_scores: Iterable[float], target_fpr: float) -> CalibrationResult:
        result = calibrate_threshold(
            negative_scores, target_fpr, comparator=Comparator.STRICT_GREATER
        )
        return CalibrationResult(
            result.threshold,
            target_fpr,
            result.negative_count,
            result.false_positive_count,
            result.empirical_calibration_exceedance,
            result.comparator,
            "CALIBRATED",
        )

    def metadata(self) -> Mapping[str, Any]:
        return {
            **asdict(self.config),
            "seeding_scheme": self.config.seeding_scheme,
            "reference_commit": KGW_REFERENCE_COMMIT,
            "decision_comparator": Comparator.STRICT_GREATER,
            "decision_comparator_authority": (
                "KGW reference detector prediction uses z_score > z_threshold"
            ),
        }

    def authority(self) -> AuthorityRecord:
        return KGW_AUTHORITY
