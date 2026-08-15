"""Official Transformers generation plus DeepMind Weighted Mean detection."""

from __future__ import annotations

from collections.abc import Iterable, Mapping
from dataclasses import asdict, dataclass
from typing import Any

import numpy as np

from waterlarp.adapters.base import (
    CalibrationResult,
    DetectionScore,
    TokenizerLike,
    WatermarkAdapter,
)
from waterlarp.authority import SYNTHID_AUTHORITY, AuthorityRecord
from waterlarp.calibration.thresholds import calibrate_threshold
from waterlarp.config import Comparator
from waterlarp.rng import derive_seed

SYNTHID_REFERENCE_COMMIT = "addb4a158143c7c6851a1308f78b89fceed59683"
TRANSFORMERS_REFERENCE_COMMIT = "5eddc12edfaf8cafde8c9bae4ccb12f8a139b4f9"


def synthid_keys(master_seed: int, depth: int = 30) -> tuple[int, ...]:
    if depth <= 0:
        raise ValueError("depth must be positive")
    keys: list[int] = []
    index = 0
    while len(keys) < depth:
        candidate = derive_seed(master_seed, f"synthid/key/{index}", bits=31)
        if candidate not in keys:
            keys.append(candidate)
        index += 1
    return tuple(keys)


@dataclass(frozen=True)
class SynthIdConfig:
    keys: tuple[int, ...]
    ngram_len: int = 5
    sampling_table_size: int = 2**16
    sampling_table_seed: int = 0
    context_history_size: int = 1024
    device: str = "cpu"
    detector: str = "weighted_mean"
    length_calibration: str = "length_specific_empirical"
    num_leaves: int = 2

    def validate(self) -> None:
        if not self.keys or len(set(self.keys)) != len(self.keys):
            raise ValueError("SynthID keys must be non-empty and unique")
        if self.ngram_len < 2 or self.sampling_table_size <= 0 or self.context_history_size <= 0:
            raise ValueError("invalid SynthID n-gram, sampling table, or context history")
        if self.detector != "weighted_mean":
            raise ValueError("Bayesian detection is secondary and not enabled in WaterLARP v1")
        if self.length_calibration != "length_specific_empirical":
            raise ValueError("v1 requires explicit length-specific calibration")


class SynthIdAdapter(WatermarkAdapter):
    def __init__(self, config: SynthIdConfig) -> None:
        config.validate()
        self.config = config
        self._processor: Any = None

    def prepare_generation(self, model: Any, tokenizer: TokenizerLike, device: str) -> Any:
        if device != self.config.device:
            raise ValueError("generation device must match SynthID sampling-table provenance")
        return self._logits_processor()

    def _logits_processor(self) -> Any:
        if self._processor is None:
            import torch
            from transformers import SynthIDTextWatermarkLogitsProcessor

            self._processor = SynthIDTextWatermarkLogitsProcessor(
                ngram_len=self.config.ngram_len,
                keys=list(self.config.keys),
                sampling_table_size=self.config.sampling_table_size,
                sampling_table_seed=self.config.sampling_table_seed,
                context_history_size=self.config.context_history_size,
                device=torch.device(self.config.device),
            )
        return self._processor

    def apply_generation(self, generation_state: Any, input_ids: Any, scores: Any) -> Any:
        return generation_state(input_ids, scores)

    @staticmethod
    def weighted_mean(g_values: np.ndarray, mask: np.ndarray) -> float:
        if g_values.ndim != 3 or mask.ndim != 2 or g_values.shape[:2] != mask.shape:
            raise ValueError("g_values and mask shapes do not match official detector semantics")
        depth = g_values.shape[-1]
        weights = np.linspace(10, 1, depth, dtype=np.float64)
        weights *= depth / weights.sum()
        denominator = depth * mask.sum(axis=1)
        if np.any(denominator == 0):
            raise ValueError("no unmasked SynthID n-grams are available for scoring")
        scores = (g_values * weights[None, None, :] * mask[:, :, None]).sum(
            axis=(1, 2)
        ) / denominator
        return float(scores[0])

    def score_token_ids(self, token_ids: list[int], tokenizer: TokenizerLike) -> DetectionScore:
        import torch

        if len(token_ids) < self.config.ngram_len:
            return DetectionScore(
                0.0,
                "synthid_weighted_mean",
                0,
                None,
                {
                    "ngram_len": self.config.ngram_len,
                    "depth": len(self.config.keys),
                    "unmasked_ngrams": 0,
                    "repetition_mask_semantics": ("transformers_synthid_context_history_v5.15.0"),
                    "weighted_mean": None,
                    "score_status": "UNSUPPORTED_NO_UNMASKED_NGRAMS",
                },
            )
        processor = self._logits_processor()
        ids = torch.tensor([token_ids], device=self.config.device)
        g_values = processor.compute_g_values(ids)
        mask = processor.compute_context_repetition_mask(ids)
        score = self.weighted_mean(g_values.cpu().numpy(), mask.cpu().numpy())
        return DetectionScore(
            score,
            "synthid_weighted_mean",
            int(mask.sum().item()),
            None,
            {
                "ngram_len": self.config.ngram_len,
                "depth": len(self.config.keys),
                "unmasked_ngrams": int(mask.sum().item()),
                "repetition_mask_semantics": "transformers_synthid_context_history_v5.15.0",
                "weighted_mean": score,
            },
        )

    def calibrate(self, negative_scores: Iterable[float], target_fpr: float) -> CalibrationResult:
        result = calibrate_threshold(
            negative_scores, target_fpr, comparator=Comparator.GREATER_OR_EQUAL
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
            "reference_commit": SYNTHID_REFERENCE_COMMIT,
            "transformers_commit": TRANSFORMERS_REFERENCE_COMMIT,
            "configuration_label": "REFERENCE CONFIGURATION",
            "decision_comparator": Comparator.GREATER_OR_EQUAL,
            "decision_comparator_authority": (
                "WaterLARP benchmark semantics; DeepMind Weighted Mean defines a score, "
                "not a tie decision"
            ),
        }

    def authority(self) -> AuthorityRecord:
        return SYNTHID_AUTHORITY
