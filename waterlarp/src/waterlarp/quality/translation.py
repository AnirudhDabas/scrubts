"""Translation metric configuration; sacreBLEU signatures must be persisted."""

from dataclasses import dataclass


@dataclass(frozen=True)
class TranslationMetricConfig:
    name: str = "sacrebleu"
    signature: str = "UNEXECUTED"
    comet_enabled: bool = False
