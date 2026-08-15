"""C4-style continuation descriptor."""

from waterlarp.datasets.base import DatasetIdentity, DatasetSample

IDENTITY = DatasetIdentity(
    "allenai/c4",
    "1588ec454efa1a09f29cd18ddd04fe05fc8653a2",
    "validation",
    "ODC-BY-1.0; source URLs retain terms",
    "high-entropy-continuation",
    "Continue the following passage without commentary:\n\n{text}\n\nContinuation:",
    1024,
)


def adapt(row: dict[str, object], index: int) -> DatasetSample:
    text = str(row["text"])
    prompt = IDENTITY.prompt_template.format(text=text[:2000])
    return DatasetSample(f"c4-validation-{index}", prompt, None, {"url": row.get("url")})
