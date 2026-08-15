"""Read-only hardware, environment, and cache inventory."""

from __future__ import annotations

import importlib.metadata
import os
import platform
import shutil
import sys
from pathlib import Path
from typing import Any


def _version(name: str) -> str | None:
    try:
        return importlib.metadata.version(name)
    except importlib.metadata.PackageNotFoundError:
        return None


def doctor_report() -> dict[str, Any]:
    report: dict[str, Any] = {
        "python": sys.version,
        "executable": sys.executable,
        "platform": platform.platform(),
        "cpu": platform.processor(),
        "logical_cpu_count": os.cpu_count(),
        "packages": {
            name: _version(name) for name in ("torch", "transformers", "datasets", "numpy", "scipy")
        },
        "model_cache": str(Path(os.getenv("HF_HOME", Path.home() / ".cache" / "huggingface"))),
        "dataset_cache": str(
            Path(
                os.getenv("HF_DATASETS_CACHE", Path.home() / ".cache" / "huggingface" / "datasets")
            )
        ),
    }
    try:
        import torch

        report.update(
            {
                "cuda_available": torch.cuda.is_available(),
                "cuda_build": torch.version.cuda,
                "gpus": [
                    {
                        "name": torch.cuda.get_device_name(index),
                        "vram_bytes": torch.cuda.get_device_properties(index).total_memory,
                    }
                    for index in range(torch.cuda.device_count())
                ],
            }
        )
    except ImportError:
        report.update({"cuda_available": False, "cuda_build": None, "gpus": []})
    usage = shutil.disk_usage(Path.cwd())
    report["workspace_disk_free_bytes"] = usage.free
    return report
