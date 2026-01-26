# Tracing and Error Handling Implementation Plan

## 1. Add Dependencies

- Update `crates/brul/Cargo.toml` to include:
  - `tracing = "0.1"`
  - `tracing-subscriber = "0.3"`
  - `thiserror = "1.0"` (for error handling)
  - `anyhow = "1.0"` (optional)

## 2. Set up Tracing Subscriber

- Create `src/tracing_setup.rs` with a function `setup_tracing()` that configures `tracing-subscriber`.
- Call `setup_tracing()` in `examples/basic-app/src/main.rs` and any other entry points.

## 3. Define Base Error Types

- Implement `crates/brul/src/error.rs`:
  - Define `BrulError` enum using `thiserror::Error`.
  - Include variants for `Io(std::io::Error)`, `Parse(std::string::ParseError)`, `Config(String)`, etc.
  - Implement `Display` and `Error` traits automatically via `thiserror`.

## 4. Convert Existing Errors to BrulError

- Update functions returning `io::Result<T>` or `Result<T, E>` to return `Result<T, BrulError>`.
- Implement `From<std::io::Error>` for `BrulError`.
- Implement `From<ConfigError>` if needed.

## 5. Propagate Errors in Library Crates

- Modify `crates/brul/src/lib.rs` to use `BrulError` in public APIs.
- Update `crates/brul/src/app/*.rs` modules to propagate `BrulError`.

## 6. Add Instrumentation with Tracing

- Insert `tracing::info!`, `tracing::debug!`, `tracing::error!` macros at key points:
  - Module imports.
  - Key functions like `Builder::build`, `Manager::start`, `Handle::process`.
  - Event bus send/receive.
  - Runtime initialization.

## 7. Update Error Handling in Example

- Modify `examples/basic-app/src/main.rs` to use `BrulError` and trace startup.

## 8. Testing

- Add unit tests for error conversion and tracing setup.

## 9. Documentation

- Update README with tracing and error handling usage.
