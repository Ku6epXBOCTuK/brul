set shell := ["powershell.exe", "-NoProfile", "-Command"]

target_dir := trim(`cargo metadata --format-version 1 --no-deps | ConvertFrom-Json | ForEach-Object target_directory`)

generate-rules:
    cargo run -p brul-syntax --bin rules

transparent: transparent-software

transparent-software:
    $env:BRUL_EXAMPLE_BACKEND = 'software'; cargo run -p transparency

transparent-gpu:
    $env:BRUL_EXAMPLE_BACKEND = 'gpu'; cargo run -p transparency

check:
    cargo fmt --all --check
    cargo clippy --workspace --all-targets -- -D warnings
    cargo nextest run --workspace
    # NOTE: nextest не запускает доктесты — их прогоняет cargo test
    cargo test --workspace --doc

cov:
    cargo llvm-cov --workspace --lcov --output-path {{target_dir}}/lcov.info --include-build-script

crap: cov
    cargo crap --lcov {{target_dir}}/lcov.info --workspace --threshold 15

mutants:
    git diff main > {{target_dir}}/phase.diff
    cargo mutants --workspace -D {{target_dir}}/phase.diff

debt: crap mutants
