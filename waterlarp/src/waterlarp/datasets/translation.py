"""Translation is configured but omitted from the local pilot."""

from waterlarp.datasets.base import DatasetIdentity

IDENTITY = DatasetIdentity(
    "Helsinki-NLP/opus_books",
    "refs/convert/parquet",
    "train",
    "varies by source corpus",
    "translation",
    "Translate from {source_language} to {target_language}:\n{text}\nTranslation:",
    512,
)
