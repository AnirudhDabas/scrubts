set windows-shell := ["powershell.exe", "-NoLogo", "-NoProfile", "-Command"]

check:
    cargo fmt --check
    cargo clippy --workspace --all-targets -- -D warnings
    cargo test --workspace

prove:
    python tools/prove.py

release-check:
    python tools/third_party_licenses.py verify
    waterlarp\.venv\Scripts\python.exe -m unittest tools.tests.test_third_party_licenses tools.tests.test_release tools.tests.test_prove

waterlarp-check:
    waterlarp\.venv\Scripts\python.exe -m ruff check --config waterlarp\pyproject.toml waterlarp\src waterlarp\tests
    waterlarp\.venv\Scripts\python.exe -m mypy --config-file waterlarp\pyproject.toml waterlarp\src
    waterlarp\.venv\Scripts\python.exe -m pytest -q -c waterlarp\pyproject.toml waterlarp\tests -m "not parity"
