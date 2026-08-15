"""Opt-in MBPP execution with process isolation and a hard timeout.

Python's ``-I`` mode and a disposable working directory are not an OS security
sandbox. Canonical runs therefore keep generated-code execution disabled unless
the operator supplies a stronger external sandbox and explicitly opts in.
"""

from __future__ import annotations

import hashlib
import subprocess
import sys
import tempfile
from dataclasses import dataclass


@dataclass(frozen=True)
class MbppExecutionResult:
    passed: bool
    tests_run: int
    timeout: bool
    stderr_sha256: str | None
    isolation: str


def execute_candidate(
    code: str,
    tests: tuple[str, ...],
    *,
    timeout_seconds: float = 2.0,
    allow_untrusted_execution: bool = False,
) -> MbppExecutionResult:
    """Execute assertions only after an explicit unsafe-code opt-in."""

    if not allow_untrusted_execution:
        raise PermissionError(
            "MBPP candidate execution requires explicit opt-in and an external sandbox"
        )
    if not tests:
        raise ValueError("at least one MBPP test is required")
    payload = code + "\n" + "\n".join(tests) + "\n"
    with tempfile.TemporaryDirectory(prefix="waterlarp-mbpp-") as directory:
        try:
            completed = subprocess.run(
                [sys.executable, "-I", "-c", payload],
                cwd=directory,
                capture_output=True,
                timeout=timeout_seconds,
                check=False,
            )
        except subprocess.TimeoutExpired as exc:
            stderr = exc.stderr or b""
            return MbppExecutionResult(
                passed=False,
                tests_run=len(tests),
                timeout=True,
                stderr_sha256=hashlib.sha256(stderr).hexdigest() if stderr else None,
                isolation="python-isolated-mode+temporary-directory;not-os-sandboxed",
            )
    return MbppExecutionResult(
        passed=completed.returncode == 0,
        tests_run=len(tests),
        timeout=False,
        stderr_sha256=(hashlib.sha256(completed.stderr).hexdigest() if completed.stderr else None),
        isolation="python-isolated-mode+temporary-directory;not-os-sandboxed",
    )
