"""MBPP low-entropy code descriptor."""

from waterlarp.datasets.base import DatasetIdentity, DatasetSample

IDENTITY = DatasetIdentity(
    "google-research-datasets/mbpp",
    "4bb6404fdc6cacfda99d4ac4205087b89d32030c",
    "test",
    "CC-BY-4.0",
    "low-entropy-code",
    "Write a Python function satisfying this task. Output code only.\n\nTask: {text}\n",
    512,
)


def adapt(row: dict[str, object], index: int) -> DatasetSample:
    identifier = row.get("task_id", index)
    return DatasetSample(
        f"mbpp-test-{identifier}",
        IDENTITY.prompt_template.format(text=row["text"]),
        str(row.get("code", "")),
        {"test_list": row.get("test_list", [])},
    )
