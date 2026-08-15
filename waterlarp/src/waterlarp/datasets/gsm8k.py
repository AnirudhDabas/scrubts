"""GSM8K medium-entropy reasoning descriptor."""

from waterlarp.datasets.base import DatasetIdentity, DatasetSample

IDENTITY = DatasetIdentity(
    "openai/gsm8k",
    "740312add88f781978c0658806c59bc2815b9866",
    "test",
    "MIT",
    "medium-entropy-math",
    "Solve the problem. End with `#### <number>`.\n\nProblem: {question}\n\nSolution:",
    512,
)


def adapt(row: dict[str, object], index: int) -> DatasetSample:
    return DatasetSample(
        f"gsm8k-test-{index}",
        IDENTITY.prompt_template.format(question=row["question"]),
        str(row["answer"]),
        {},
    )
