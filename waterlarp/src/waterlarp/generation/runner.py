"""Autoregressive generation with immutable pre-processor logit observation."""

from __future__ import annotations

import hashlib
import time
from collections.abc import Callable, Sequence
from dataclasses import dataclass
from typing import Any

import numpy as np
import torch

from waterlarp.entropy.metrics import shannon_entropy_from_logits, spike_entropy_from_logits


@dataclass(frozen=True)
class GenerationStep:
    token_id: int
    base_logits_sha256: str
    shannon_entropy: float
    spike_entropy: float


@dataclass(frozen=True)
class GeneratedExample:
    prompt_token_ids: tuple[int, ...]
    generated_token_ids: tuple[int, ...]
    text: str
    steps: tuple[GenerationStep, ...]
    runtime_seconds: float


StepObserver = Callable[[int, torch.Tensor, torch.Tensor], None]


def require_explicit_model_inputs(model: Any, tokenizer: Any, generator: Any) -> None:
    if model is None or tokenizer is None or generator is None:
        raise ValueError("model, tokenizer, and seeded generator must be supplied explicitly")


def _logits_hash(logits: torch.Tensor) -> str:
    values = logits.detach().to(device="cpu", dtype=torch.float32).contiguous().numpy()
    descriptor = f"float32:{','.join(str(value) for value in values.shape)}\0".encode()
    return hashlib.sha256(descriptor + values.tobytes(order="C")).hexdigest()


def _sample_logits(
    logits: torch.Tensor,
    *,
    temperature: float,
    top_k: int,
    top_p: float,
    generator: torch.Generator,
) -> torch.Tensor:
    if temperature <= 0 or top_k < 0 or not 0 < top_p <= 1:
        raise ValueError("invalid temperature, top_k, or top_p")
    filtered = logits / temperature
    if top_k:
        keep = min(top_k, filtered.shape[-1])
        boundary = torch.topk(filtered, keep, dim=-1).values[..., -1, None]
        filtered = filtered.masked_fill(filtered < boundary, float("-inf"))
    if top_p < 1:
        sorted_logits, sorted_indices = torch.sort(filtered, descending=True, dim=-1)
        cumulative = torch.cumsum(torch.softmax(sorted_logits, dim=-1), dim=-1)
        remove = cumulative > top_p
        remove[..., 1:] = remove[..., :-1].clone()
        remove[..., 0] = False
        sorted_logits = sorted_logits.masked_fill(remove, float("-inf"))
        filtered = torch.full_like(filtered, float("-inf")).scatter(
            -1, sorted_indices, sorted_logits
        )
    probabilities = torch.softmax(filtered, dim=-1)
    return torch.multinomial(probabilities, num_samples=1, generator=generator)


def generate_autoregressive(
    *,
    model: Any,
    tokenizer: Any,
    prompt: str,
    seed: int,
    max_new_tokens: int,
    processors: Sequence[Callable[[torch.Tensor, torch.Tensor], torch.Tensor]] = (),
    temperature: float = 0.8,
    top_k: int = 40,
    top_p: float = 0.95,
    device: str = "cpu",
    observer: StepObserver | None = None,
) -> GeneratedExample:
    """Capture ``model(...).logits[:, -1, :]`` before every downstream processor.

    The captured tensor is detached and cloned. Processors receive a second clone,
    so in-place mutation or replacement cannot alter the entropy input. The model
    is conditioned on the prompt plus every token sampled before the observed step.
    """

    if max_new_tokens <= 0:
        raise ValueError("max_new_tokens must be positive")
    encoded = tokenizer(prompt, return_tensors="pt")
    sequence = encoded.input_ids.to(device)
    prompt_ids = tuple(int(value) for value in sequence[0].tolist())
    generator = torch.Generator(device=device)
    generator.manual_seed(seed)
    steps: list[GenerationStep] = []
    generated: list[int] = []
    past_key_values: Any = None
    started = time.perf_counter()
    with torch.no_grad():
        for step_index in range(max_new_tokens):
            model_input = sequence if past_key_values is None else sequence[:, -1:]
            output = model(
                input_ids=model_input,
                past_key_values=past_key_values,
                use_cache=True,
                return_dict=True,
            )
            past_key_values = output.past_key_values
            # This is the actual base-model next-token distribution at the current
            # conditioning context. It is not a Transformers generation output field.
            base_snapshot = output.logits[:, -1, :].detach().clone()
            entropy_input = base_snapshot.to(device="cpu", dtype=torch.float64).numpy()
            shannon = float(shannon_entropy_from_logits(entropy_input)[0])
            spike = float(spike_entropy_from_logits(entropy_input)[0])
            processed = base_snapshot.clone()
            for processor in processors:
                processed = processor(sequence, processed)
            if observer is not None:
                observer(step_index, base_snapshot.clone(), processed.detach().clone())
            next_token = _sample_logits(
                processed,
                temperature=temperature,
                top_k=top_k,
                top_p=top_p,
                generator=generator,
            )
            token_id = int(next_token.item())
            generated.append(token_id)
            steps.append(GenerationStep(token_id, _logits_hash(base_snapshot), shannon, spike))
            sequence = torch.cat((sequence, next_token), dim=-1)
    decoded = tokenizer.decode(generated, skip_special_tokens=True)
    if not isinstance(decoded, str):
        raise TypeError("single generated sequence decode did not return text")
    return GeneratedExample(
        prompt_ids,
        tuple(generated),
        decoded,
        tuple(steps),
        time.perf_counter() - started,
    )


def generate_with_observer(
    *,
    model: Any,
    tokenizer: Any,
    seeded_generator: Any,
    run: Callable[[Any, Any, Any], GeneratedExample],
) -> GeneratedExample:
    require_explicit_model_inputs(model, tokenizer, seeded_generator)
    return run(model, tokenizer, seeded_generator)


def entropy_from_tensor(logits: torch.Tensor) -> tuple[float, float]:
    """Small independent-test helper using the same frozen formulas."""

    values = np.asarray(logits.detach().cpu(), dtype=np.float64)
    return (
        float(shannon_entropy_from_logits(values)[0]),
        float(spike_entropy_from_logits(values)[0]),
    )
