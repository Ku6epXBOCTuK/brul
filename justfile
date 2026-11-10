check:
    cargo fmt --all --check
    cargo clippy --workspace --all-targets -- -D warnings
    cargo test --workspace

cov:
    cargo llvm-cov --workspace --lcov --output-path target/lcov.info --include-build-script

crap: cov
    cargo crap --lcov target/lcov.info --workspace --threshold 15

mutants:
    git diff main > target/phase.diff
    cargo mutants --workspace -D target/phase.diff

debt: crap mutants
