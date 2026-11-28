# Product

BRUL is a framework for native desktop applications in Rust. The product is the
framework. Applications must be fast to write, easy to read, and fast to run.

## Framework requirements

Review checks three requirements on every task. There is no separate stage for
them.

### DX

Application development must have ideal DX. Use as little boilerplate as
possible. Provide reactivity out of the box. Allow only magic that a developer
can understand. Do not keep a rule that one sentence cannot explain.

Build developer tools on the same foundation. Do not attach them later. LSP, the
formatter, and the linter grow from the same AST and styles as codegen.
Therefore the DSL must parse without execution.

Move boilerplate that repeats across applications into `brul`. Do not copy it.

### Rust-way

The framework and the `.rs` files it generates read as ordinary Rust. Use a type
at a boundary. Match on an enum. Return a type, not `String`, from a public
boundary.

A module from codegen passes the same review as hand-written code. Names come
from the source. There are no numbered wrappers and no baked-in ASTs. Code
preferences are in [AGENTS.md](../../AGENTS.md#rust-way).

### Performance

Speed of the resulting applications is part of the contract, not an optimization
for later. Cold start, memory, and frame time are part of acceptance. Reactivity
must not trigger a redraw that the data did not cause.

Measurements and comparison rules are in [benchmarks.md](benchmarks.md).

## Example

`examples/hello` is a consumer in the working copy. It has one window, one host
command, a kit, and a layout test. See
[its README](../../examples/hello/README.md) for its contents.

## Maturity criteria

The framework is mature when these three requirements hold in real scenarios. An
application builds and starts without boilerplate beyond `build.rs`, `main.rs`,
and `.brul`. It reads as ordinary Rust. It passes cold start, memory, and frame
measurements. Tracker tasks show what is verified and what remains.
