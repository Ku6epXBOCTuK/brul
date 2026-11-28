# Measurements

A snapshot is from one machine. It is not an acceptance criterion. Compare only
runs on the same machine and toolchain. Scenarios and code are in
`crates/brul-reactive/benches/reactive.rs`, `benches/each.rs`, and
`benches/read.rs`. The core contract is in
[architecture.md, "Reactive core"](architecture.md#reactive-core).

All snapshots below come from one machine:

- CPU: Intel Core i7-8700K, 6 cores and 12 threads.
- OS: Windows 10 Home, 10.0.19045, x86_64.
- Toolchain: `nightly-2026-10-15`, rustc 1.101.0-nightly (7df81381d 2026-10-14),
  cargo 1.101.0-nightly.
- Criterion mode: 50 samples, 1 s warm-up, 2 s measurement for `brul-reactive`
  and 3 s for `brul`.

Time is the Criterion estimate. Spread and outliers stay in the run report.

## 2026-11-17, after the performance series (BRUL-415 to BRUL-489)

Runs use the same machine and toolchain as before. This snapshot replaces two
earlier sections on the core and on reads. Earlier steps of the series stay in
the file history.

`cargo bench -p brul-reactive`

| Scenario         | Size |    Time |
| ---------------- | ---: | ------: |
| chain            |   10 |  637 ns |
| chain            |  100 | 5.43 µs |
| chain            | 1000 | 55.6 µs |
| diamond          |    — |  262 ns |
| many_subscribers |   10 |  649 ns |
| many_subscribers |  100 | 11.3 µs |
| many_subscribers | 1000 |  557 µs |
| batched_writes   |   10 |  410 ns |
| batched_writes   |  100 | 3.69 µs |

`cargo bench -p brul --bench read`

| Scenario                  |    Time |
| ------------------------- | ------: |
| read/property_string      | 32.6 ns |
| read/property_string_id   | 20.9 ns |
| read/property_scalar      | 46.1 ns |
| read/property_scalar_id   | 25.6 ns |
| read/variable_in_frame    | 41.7 ns |
| read/variable_in_frame_id | 24.5 ns |
| binding/props_read        |  138 ns |
| binding/props_read_id     | 87.9 ns |
| binding/frame_only        | 12.4 ns |

`cargo bench -p brul --bench each`

| Scenario                   |    Time |
| -------------------------- | ------: |
| each_row/scopes_6_bindings | 1.16 µs |
| each_row/widgets           |  888 ns |
| build_scopes/depth_1       | 10.0 ns |
| build_scopes/depth_3       | 9.73 ns |
| apply_row/declares         | 12.8 ns |
| row_match/keyed_100        | 27.4 µs |
| row_match/keyed_1000       |  296 µs |

`cargo bench -p brul --bench layout`

| Scenario             |    Time |
| -------------------- | ------: |
| layout/window        | 83.4 µs |
| layout/synthetic-500 |  551 µs |

`row_match/keyed_N` builds a `RowMatcher` over N rows and makes one lookup per
row. That is 0.27 µs per row at 100 rows and 0.30 µs at 1000 rows. The cost does
not depend on N. The hash uses the row key. Equality checks the element only in
the found bucket.

What changed since the last snapshot:

- BRUL-415: a property is read through its own signal, not through a shared
  component revision. `chain/1000` went from 182 to 55.6 µs. `diamond` went from
  0.99 to 0.26 µs. `batched_writes/100` went from 16.5 to 3.69 µs. The cost is
  an environment variable read. It subscribes to the coarse `external` signal,
  so the read became a signal read. `read/variable_in_frame_id` went from 10.3
  to 24.5 ns. The control `binding/frame_only` grew by 10%. Do not read
  differences below 3 ns.
- BRUL-416, BRUL-485, and BRUL-489: the arena reuses slots. Marking uses its own
  stack and queue. The dependency list does not lose capacity. The bench does
  not allocate a push. `brul-reactive/tests/alloc_notify.rs` and the rebuild
  section below hold that guarantee.
- BRUL-419 and BRUL-118: a matching `each` row is not built at all. The key is
  hashed separately from the element. For `row_match`, 100 rows went from 81.0
  to 27.4 µs and 1000 rows went from 765 to 296 µs. Earlier `each_row/*` results
  stayed as they were.

Two runs of `read` and `each` differ by up to 15% across rows. Run noise is
above a difference of a few nanoseconds.

## Allocation counter in an application (BRUL-232)

The counter is enabled for an application build. It lives in the generated
module:

```powershell
$env:BRUL_ALLOC_COUNT = "1"
cargo build --release
```

Without the variable, the block is cut out by `#[cfg(brul_alloc_count)]`. With
the variable enabled, `alloc` and `realloc` are counted separately. `dealloc`
only passes through. Bytes are the requested size of each call.

Counters reset after the first frame. Window and font initialization does not
mix with the UI. The result goes to the application log at
`~/.config/brul/<exe>.log` as a line of this form:

```text
brul allocations: init <allocs> allocs, <reallocs> reallocs, <bytes> bytes; ui ...
```

`init` is startup before the first frame. `ui` is everything after it. The line
is printed only on exit through the event loop. A killed process gives no
report.

## 2026-11-16, cold start and config (BRUL-196)

Cold start method: run `cargo build --release`, then start the application five
times with `$env:BRUL_STARTUP_LOG = "1"`. Stop the process after 2.5 s. Take the
line `brul startup: first frame in ...` from `~/.config/brul/<exe>.log`.

The measurement runs from entry into the event loop (`run_windows`) to the first
shown frame. It includes window creation, GPU context, fonts, and tree building.
EXE loading and log initialization are not included. Add process startup to get
the "time from click". Only `BRUL_STARTUP_LOG` prints the line. By default there
is no line. An elevated process does not inherit the caller's environment. Set
the variable in the user profile.

Config check method: replace the file with broken TOML (`unclosed array`). Start
the application. Capture the exit code and the tail of the log. The parse error
and line number go to `~/.config/brul/<exe>.log` at `WARN` level. The
application starts with defaults. The broken file is not overwritten. The next
settings save replaces it.

## 2026-11-17, DPI: text and atlas (BRUL-196)

The monitor scale did not change. Both screens are 2560×1440 at 100%. winit
takes scale only from the system. One build was measured with
`cargo bench -p brul --bench dpi`: 16 UI rows at four sizes (14/16/18/24), at
scales 1.00/1.25/1.50/2.00.

| Scale | Cold layout | Repeat | Atlas entries |
| ----- | ----------: | -----: | ------------: |
| 1.00  |     11.6 ms | 651 µs |           428 |
| 1.25  |     11.3 ms | 627 µs |           430 |
| 1.50  |     13.1 ms | 631 µs |           425 |
| 2.00  |     13.1 ms | 649 µs |           427 |

Cold layout uses a new `TextEngine` on each iteration. It shapes and rasterizes
all 64 "row × size" pairs. Font loading is not included. Iteration setup creates
the engine.

Repeat uses the same rows against a ready atlas.

What the data shows:

- Rasterization at 150–200% costs 13% more. There are the same glyphs, but each
  is larger.
- Repeat layout does not depend on scale. It is atlas hits.
- A 1024×1024 atlas holds the whole set even at 200%. That is 4 MiB of RGBA at
  any scale.

Not measured: real window startup at a fractional scale. That needs a screen
with a setting other than 100%. Tests in `brul-gui` (text metrics) and
`brul-window` (surface and frame) hold that surface and layout follow
`config × scale`.

## 2026-11-17, rebuild without arena growth (BRUL-416)

A temporary test counts allocations in the global allocator. It makes 2000
rebuilds. Each has a scope with ten effects and `EffectScope::dispose`. `alloc`
and `realloc` are counted separately. Bytes are the requested size.

| Build         | Allocs | Reallocs |   Bytes |
| ------------- | -----: | -------: | ------: |
| without reuse |  66003 |     4015 | 6462176 |
| reuse         |  66004 |     4006 | 1746000 |

The call counts are the same. An effect and its closure are allocated as before.
What disappears is `Vec<Node>` growth. 20 000 created and stopped effects
request 1.7 MiB instead of 6.5 MB.

## 2026-11-18, example start on two backends (BRUL-530)

One `hello` binary switches backends through `BRUL_EXAMPLE_BACKEND`, which is
`software` or `gpu`. It writes to `WindowConfig.backend` before `run_generated`.

`docs/dev/scripts/measure-startup.ps1` makes the measurement: build with
`cargo build --release -p hello`, run five times per backend, delete the log
before each run, wait for the line
`brul startup: first frame in ... ms, backend ...`, then stop the process. The
binary path comes from `cargo metadata`. In this copy the target directory is
`F:/Cache/cargo-target/brul`.

```powershell
.\docs\dev\scripts\measure-startup.ps1
```

| Backend  | First frame, median |        Min / max | From start to line, median |
| -------- | ------------------: | ---------------: | -------------------------: |
| software |             69.7 ms |   63.1 / 75.1 ms |                   193.4 ms |
| gpu      |            629.9 ms | 602.9 / 689.0 ms |                   746.6 ms |

The inner mark runs from entry into the event loop to the first frame. The outer
mark runs from process start to the log line.

The software path does not search for an adapter and does not create a GPU
context. It reaches the first frame about 9 times faster.

The numbers come from one machine. They do not transfer to another. An elevated
process does not inherit the caller's environment. Set `BRUL_STARTUP_LOG` and
`BRUL_EXAMPLE_BACKEND` in the user profile. Frame equality between the two paths
was not checked. Only time to first frame was compared.

## 2026-11-18, GPU frame against CPU frame (BRUL-537, BRUL-546)

This dev check does not run in CI. The adapter is not reliable, so compare on a
machine with a window. Build and run:

```powershell
cargo build --release -p hello
.\docs\dev\scripts\compare-frames.ps1
```

The script starts `hello` on the `software` and `gpu` backends in turn.
`BRUL_STARTUP_LOG=1` makes the application redraw every 16 ms and print startup
and raster marks. `BRUL_FRAME_DUMP` sets the path to an RGBA frame after 60
redraws, when animations have settled. `BRUL_LOG_FILE` sends the log to
`target/frame-compare` instead of the user directory. The script closes the
window with `CloseMainWindow` so the raster mark can be written. Then it swaps R
and B in the GPU dump if the surface is BGRA. It counts a byte-by-byte
difference.

| Backend  | First frame | Raster first | Raster steady, median | Frames |
| -------- | ----------: | -----------: | --------------------: | -----: |
| software |      192 ms |       7.2 ms |                8.5 ms |    384 |
| gpu      |     1279 ms |      10.8 ms |               16.5 ms |    241 |

`raster steady` is `renderer.render` time without the first frame.

A repeat run gives the same order. The software backend starts faster. It draws
a steady frame more slowly.

## How to update

A new snapshot replaces the old one. Git keeps history. A run on another machine
or toolchain opens its own section with its own header. Do not compare numbers
from different sections.
