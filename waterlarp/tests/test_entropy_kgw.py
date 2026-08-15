from pathlib import Path
from types import SimpleNamespace

import pytest
import torch

from waterlarp.adapters.kgw import KgwAdapter, KgwConfig
from waterlarp.generation.runner import entropy_from_tensor, generate_autoregressive


class KgwTokenizer:
    def get_vocab(self) -> dict[str, int]:
        return {str(index): index for index in range(64)}

    def __call__(self, text: str, **_: object) -> SimpleNamespace:
        return SimpleNamespace(input_ids=torch.tensor([[int(value) for value in text.split()]]))

    def decode(self, token_ids: object, **_: object) -> str:
        return " ".join(str(value) for value in token_ids)


class KgwTinyModel:
    def __init__(self) -> None:
        self.contexts: list[torch.Tensor] = []

    @staticmethod
    def oracle(context: torch.Tensor) -> torch.Tensor:
        logits = torch.linspace(-2.0, 2.0, 64).unsqueeze(0)
        logits[0, int(context.sum().item()) % 64] += 1.0
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


@pytest.mark.parity
def test_real_kgw_processor_cannot_mutate_recorded_base_entropy(
    kgw_checkout: Path | None,
) -> None:
    if kgw_checkout is None:
        pytest.skip("pinned KGW checkout required")
    tokenizer = KgwTokenizer()
    model = KgwTinyModel()
    adapter = KgwAdapter(KgwConfig(base_key=4182307207024115832), kgw_checkout)
    processor = adapter.prepare_generation(model, tokenizer, "cpu")
    observed: list[tuple[torch.Tensor, torch.Tensor]] = []
    generated = generate_autoregressive(
        model=model,
        tokenizer=tokenizer,
        prompt="1 2 3 4",
        seed=23,
        max_new_tokens=3,
        processors=(processor,),
        top_k=0,
        top_p=1.0,
        observer=lambda _, base, processed: observed.append((base, processed)),
    )
    assert any(not torch.equal(base, processed) for base, processed in observed)
    for context, step, (base, processed) in zip(
        model.contexts, generated.steps, observed, strict=True
    ):
        oracle = model.oracle(context)
        oracle_shannon, oracle_spike = entropy_from_tensor(oracle)
        post_shannon, _ = entropy_from_tensor(processed)
        assert torch.equal(base, oracle)
        assert step.shannon_entropy == oracle_shannon
        assert step.spike_entropy == oracle_spike
        assert step.shannon_entropy != post_shannon
