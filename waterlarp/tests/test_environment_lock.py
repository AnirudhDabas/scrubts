from __future__ import annotations

import subprocess
import sys
from pathlib import Path

PACKAGE_ROOT = Path(__file__).resolve().parents[1]


def test_dependency_lock_has_no_local_checkout_binding() -> None:
    lock = (PACKAGE_ROOT / "requirements-lock.txt").read_text(encoding="utf-8")
    lowered = lock.lower()
    assert "-e " not in lowered
    assert "file://" not in lowered
    assert "c:\\" not in lowered
    assert "/users/" not in lowered
    assert all("==" in line for line in lock.splitlines() if line and not line.startswith("#"))


def test_dependency_lock_resolves_from_a_different_path(tmp_path: Path) -> None:
    relocated = tmp_path / "relocated-checkout" / "waterlarp-requirements-lock.txt"
    relocated.parent.mkdir(parents=True)
    relocated.write_bytes((PACKAGE_ROOT / "requirements-lock.txt").read_bytes())
    base_python = Path(sys.base_prefix) / (
        "python.exe" if sys.platform == "win32" else "bin/python"
    )
    completed = subprocess.run(
        [
            str(base_python),
            "-m",
            "pip",
            "--python",
            sys.prefix,
            "install",
            "--disable-pip-version-check",
            "--dry-run",
            "--no-index",
            "--requirement",
            str(relocated),
        ],
        check=False,
        capture_output=True,
        text=True,
    )
    assert completed.returncode == 0, completed.stderr
    assert "Requirement already satisfied" in completed.stdout
