"""Pinned local semantic-operation specifications; execution is optional."""

from dataclasses import dataclass


@dataclass(frozen=True)
class SemanticTransformSpec:
    name: str
    model_repo: str
    model_revision: str
    prompt_template_sha256: str
    decoding: dict[str, int | float | bool]
    seeds: tuple[int, ...]
    executed: bool = False
    unexecuted_reason: str | None = "No compatible local operation model was run in the CPU pilot."
