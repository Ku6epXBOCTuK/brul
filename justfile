set shell := ["powershell.exe", "-NoProfile", "-Command"]

target_dir := trim(`cargo metadata --format-version 1 --no-deps | ConvertFrom-Json | ForEach-Object target_directory`)

generate-rules:
    cargo run -p brul-syntax --bin rules

register-bropicker:
    cargo build --release -p bropicker
    powershell -NoProfile -ExecutionPolicy Bypass -File apps/bropicker/tools/register.ps1

unregister-bropicker:
    powershell -NoProfile -ExecutionPolicy Bypass -File apps/bropicker/tools/register.ps1 -Uninstall

build-nobs:
    cargo build --release -p note-obs

install-nobs: build-nobs
    New-Item -ItemType Directory -Force -Path "$HOME\bin" | Out-Null
    Get-Process nobs -ErrorAction SilentlyContinue | Stop-Process -Force; Start-Sleep -Milliseconds 400
    Copy-Item -Force "{{target_dir}}\release\nobs.exe" "$HOME\bin\nobs.exe"
    Write-Output "installed: $HOME\bin\nobs.exe"

uninstall-nobs:
    Remove-Item -Force "$HOME\bin\nobs.exe" -ErrorAction SilentlyContinue; Write-Output "removed: $HOME\bin\nobs.exe"

autostart-nobs:
    $value = '"' + (Join-Path $HOME 'bin\nobs.exe') + '" --tray'; New-ItemProperty -Path "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run" -Name nobs -Value $value -PropertyType String -Force | Out-Null; Write-Output "autostart: $value"

unautostart-nobs:
    Remove-ItemProperty -Path "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run" -Name nobs -ErrorAction SilentlyContinue; Write-Output "autostart removed"

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
