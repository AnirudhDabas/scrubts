import math
from types import SimpleNamespace

import numpy as np
import torch

from waterlarp.entropy.metrics import (
    ENTROBENCH_Z,
    shannon_entropy_from_logits,
    spike_entropy_from_logits,
)
from waterlarp.generation.runner import entropy_from_tensor, generate_autoregressive


class TinyTokenizer:
    def __call__(self, text: str, **_: object) -> SimpleNamespace:
        return SimpleNamespace(input_ids=torch.tensor([[int(value) for value in text.split()]]))

    def decode(self, token_ids: object, **_: object) -> str:
        return " ".join(str(value) for value in token_ids)


class TinyModel:
    def __init__(self, vocab_size: int = 5) -> None:
        self.vocab_size = vocab_size
        self.contexts: list[torch.Tensor] = []

    def oracle(self, context: torch.Tensor) -> torch.Tensor:
        logits = torch.linspace(-1.0, 1.0, self.vocab_size).unsqueeze(0)
        logits[0, int(context.sum().item()) % self.vocab_size] += 2.5
        return logits

    def __call__(
        self, *, input_ids: torch.Tensor, past_key_values: object, **_: object
    ) -> SimpleNamespace:
        context = (
            input_ids.clone()
            if past_key_values is None
            else torch.cat((past_key_values, input_ids), dim=-1)
        )
        self.contexts.append(context.clone())
        return SimpleNamespace(
            logits=self.oracle(context).unsqueeze(1), past_key_values=context.clone()
        )


class InPlaceBias:
    def __init__(self) -> None:
        self.before: list[torch.Tensor] = []
        self.after: list[torch.Tensor] = []

    def __call__(self, _: torch.Tensor, scores: torch.Tensor) -> torch.Tensor:
        self.before.append(scores.clone())
        scores[:, 0] += 10.0
        self.after.append(scores.clone())
        return scores


def test_uniform_entropy_values() -> None:
    logits = np.zeros((1, 2), dtype=np.float64)
    assert np.isclose(shannon_entropy_from_logits(logits)[0], math.log(2))
    expected_spike = 1 / (1 + ENTROBENCH_Z / 2)
    assert np.isclose(spike_entropy_from_logits(logits)[0], expected_spike)


def test_entropy_uses_supplied_base_logits() -> None:
    base = np.array([[0.0, 0.0]])
    biased = np.array([[10.0, 0.0]])
    assert shannon_entropy_from_logits(base)[0] > shannon_entropy_from_logits(biased)[0]


def test_actual_generation_preserves_pre_processor_logits_against_in_place_mutation() -> None:
    model = TinyModel()
    processor = InPlaceBias()
    observed: list[tuple[torch.Tensor, torch.Tensor]] = []
    generated = generate_autoregressive(
        model=model,
        tokenizer=TinyTokenizer(),
        prompt="1 2 3 4",
        seed=17,
        max_new_tokens=3,
        processors=(processor,),
        top_k=0,
        top_p=1.0,
        observer=lambda _, base, processed: observed.append((base, processed)),
    )
    assert len(model.contexts) == len(generated.steps) == 3
    assert any(
        not torch.equal(before, after)
        for before, after in zip(processor.before, processor.after, strict=True)
    )
    post_entropies = []
    for index, (context, step) in enumerate(zip(model.contexts, generated.steps, strict=True)):
        oracle = model.oracle(context)
        oracle_shannon, oracle_spike = entropy_from_tensor(oracle)
        post_shannon, _ = entropy_from_tensor(processor.after[index])
        post_entropies.append(post_shannon)
        assert torch.equal(processor.before[index], oracle)
        assert torch.equal(observed[index][0], oracle)
        assert torch.equal(observed[index][1], processor.after[index])
        assert step.shannon_entropy == oracle_shannon
        assert step.spike_entropy == oracle_spike
    assert any(
        step.shannon_entropy != post
        for step, post in zip(generated.steps, post_entropies, strict=True)
    )
