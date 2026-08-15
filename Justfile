set windows-shell := ["powershell.exe", "-NoLogo", "-NoProfile", "-Command"]

check:
    cargo fmt --check
    cargo clippy --workspace --all-targets -- -D warnings
    cargo test --workspace

waterlarp-check:
    waterlarp\.venv\Scripts\python.exe -m ruff check --config waterlarp\pyproject.toml waterlarp\src waterlarp\tests
    waterlarp\.venv\Scripts\python.exe -m mypy --config-file waterlarp\pyproject.toml waterlarp\src
    waterlarp\.venv\Scripts\python.exe -m pytest -q -c waterlarp\pyproject.toml waterlarp\tests -m "not parity"
