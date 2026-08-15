import json
from pathlib import Path

import pytest

from waterlarp.adapters.kgw import KgwAdapter, KgwConfig


class FixtureTokenizer:
    def get_vocab(self) -> dict[str, int]:
        return {str(index): index for index in range(64)}

    def encode(self, text: str, **_: object) -> list[int]:
        return [int(value) for value in text.split()]

    def decode(self, token_ids: object, **_: object) -> str:
        return " ".join(str(value) for value in token_ids)


@pytest.mark.parity
def test_pinned_kgw_cpu_reference(kgw_checkout: Path | None) -> None:
    if kgw_checkout is None:
        pytest.skip("pass --kgw-checkout or WATERLARP_KGW_CHECKOUT for pinned parity")
    import torch

    fixture = json.loads((Path(__file__).parent / "fixtures/kgw_cpu_reference.json").read_text())
    adapter = KgwAdapter(KgwConfig(base_key=fixture["base_key"]), kgw_checkout)
    tokenizer = FixtureTokenizer()
    processor = adapter.prepare_generation(None, tokenizer, "cpu")
    processor.rng = torch.Generator(device="cpu")
    greenlists = [
        processor._score_rejection_sampling(
            torch.tensor(context), torch.arange(64, dtype=torch.float32)
        ).tolist()
        for context in fixture["contexts"]
    ]
    assert greenlists == fixture["greenlists"]
    score = adapter.score_token_ids(fixture["sequence"], tokenizer)
    assert score.scored_units == fixture["score"]["num_tokens_scored"]
    assert score.raw_evidence["num_green_tokens"] == fixture["score"]["num_green_tokens"]
    assert score.score == pytest.approx(fixture["score"]["z_score"], abs=1e-15)
    assert score.p_value == pytest.approx(fixture["score"]["p_value"], abs=1e-15)
    unique_sequence = list(range(20))
    unique_score = adapter.score_token_ids(unique_sequence, tokenizer)
    assert unique_score.scored_units > score.scored_units
    assert len(unique_sequence) == len(fixture["sequence"])
