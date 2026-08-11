set windows-shell := ["powershell.exe", "-NoLogo", "-NoProfile", "-Command"]

check:
    cargo fmt --check
    cargo clippy --workspace --all-targets -- -D warnings
    cargo test --workspace
