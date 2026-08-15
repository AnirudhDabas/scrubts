import json
from pathlib import Path

import pytest

from waterlarp.adapters.synthid import SynthIdAdapter, SynthIdConfig


@pytest.mark.parity
def test_pinned_synthid_cpu_reference() -> None:
    torch = pytest.importorskip("torch")
    transformers = pytest.importorskip("transformers")
    fixture = json.loads(
        (Path(__file__).parent / "fixtures/synthid_cpu_reference.json").read_text()
    )
    processor = transformers.SynthIDTextWatermarkLogitsProcessor(
        ngram_len=5,
        keys=fixture["keys"],
        sampling_table_size=2**16,
        sampling_table_seed=0,
        context_history_size=1024,
        device=torch.device("cpu"),
    )
    ids = torch.tensor([fixture["sequence"]])
    g_values = processor.compute_g_values(ids).numpy()
    mask = processor.compute_context_repetition_mask(ids).numpy()
    assert g_values[0].tolist() == fixture["g_values"]
    assert mask[0].tolist() == fixture["mask"]
    assert SynthIdAdapter.weighted_mean(g_values, mask) == pytest.approx(fixture["weighted_mean"])
    assert int(mask.sum()) < mask.shape[1]
    unique_ids = torch.tensor([list(range(len(fixture["sequence"])))])
    unique_mask = processor.compute_context_repetition_mask(unique_ids)
    assert int(unique_mask.sum()) > int(mask.sum())
    SynthIdConfig(keys=tuple(fixture["keys"])).validate()
