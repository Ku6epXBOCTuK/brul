generate-rules:
    cargo run -p brul-syntax --bin rules

register-bropicker:
    cargo build --release -p bropicker
    powershell -NoProfile -ExecutionPolicy Bypass -File apps/bropicker/tools/register.ps1

unregister-bropicker:
    powershell -NoProfile -ExecutionPolicy Bypass -File apps/bropicker/tools/register.ps1 -Uninstall

check:
    cargo fmt --all --check
    cargo clippy --workspace --all-targets -- -D warnings
    cargo nextest run --workspace
    # NOTE: nextest не запускает доктесты — их прогоняет cargo test
    cargo test --workspace --doc

cov:
    cargo llvm-cov --workspace --lcov --output-path target/lcov.info --include-build-script

crap: cov
    cargo crap --lcov target/lcov.info --workspace --threshold 15

mutants:
    git diff main > target/phase.diff
    cargo mutants --workspace -D target/phase.diff

debt: crap mutants
