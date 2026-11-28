# Process

Work is split into tasks. Review sees each change as a whole. One error belongs
to one task.

## Task

A task is one checkable behavior. A reviewer accepts or rejects it in one diff
read. A task description is self-contained. It does not need a plan.

Task boundaries:

- Do not include changes without which the result does not work.
- Do not mix behavior with mechanics. File moves, renames, and crate moves are
  separate tasks, even when many files are involved.
- Do not format or fix nearby code in passing.

A mechanical change may touch any number of files. State its volume before you
start, not after.

Remove complexity before you add it. If a task is solved by deletion, solve it
by deletion. Do not treat a symptom with a guard. Reproduce it, find the cause,
and fix the cause. If two attempts on the same premise fail in the same way, do
not write a third. State the premise in one sentence and fix it.

A task ends with a finished result, not with a first diff. Fixes and follow-up
work belong to the same task. A review fix, a refined wording, and a follow-up
in the same area stay in the current task. A new number is for separate
checkable behavior only. A change that does not alter behavior or the task
contract does not open a task.

Checks: `just check` runs formatting, clippy, and workspace tests. What does not
fit goes into a new tracker task, not into the current one.

## Guidelines

Treat other libraries and applications as a source of good practice, not as a
template. See how they solve a problem. Do not copy their API and structure into
BRUL.

Application tests check behavior: configuration, selection, whether generated
code builds and works. Numbers from codegen internals, such as structures,
bindings, children, and props, belong to engine tests. A snapshot that breaks on
a markup edit tests nothing.

## Work order

Do first what becomes cheaper if you do it now. Remove coupling first. Then
remove the daily tax. Then improve readability. Then do cosmetic work.

## Rough edges

File a rough edge in the BRUL tracker as soon as you find it in work. State what
you had to write instead of the obvious code, where it is, and how to see it. A
workaround does not replace the report, and the report names what was bypassed.
`just check` covers crates and `examples/`.

## Debt

Do not fix debt in passing. A side cleanup is a second sweep in one review.

Recipes for measurements are in the [justfile](../../justfile):

- `just cov` writes coverage to `target/lcov.info`.
- `just crap` recomputes coverage and compares CRAP with the threshold of 15.
- `just mutants` runs mutation testing on the diff from `main`.
- `just debt` runs all of them.

- CRAP ≥ 15 means a function is complex and weakly covered. Split it or cover it
  with a test. At full coverage CRAP equals cyclomatic complexity, so the
  threshold is a complexity ceiling. A value of 15 keeps parse tables with a
  match of 10 to 14 branches, where splitting is not useful. The first exception
  is winit and wgpu entry points: `ApplicationHandler`, `Gpu::new`,
  `Gpu::present_frame`, `run`, and `run_tree_themed`. They need a real window,
  an event loop, and a GPU, so nothing can cover them. Keep logic outside them
  in testable functions. CRAP findings on the entry points themselves are not
  debt. The second exception is dispatcher tables over enums: `Error::fmt` with
  18 variants and `eval_expression` with 15 expression kinds. Their complexity
  equals the number of variants and grows only with it. They live in the `allow`
  list of `.cargo-crap.toml`, not under the threshold. Raising the threshold for
  them would require a new raise with every variant. Allow entries are matched
  by function name across the workspace, so `*::fmt` covers all `Display`
  implementations. That is intended: each error message stays a table of
  strings. Build scripts are measured with `--include-build-script`. Without the
  flag cargo does not pass `RUSTFLAGS` to them, and `build.rs` appears in the
  report with zero coverage although every build runs it.
- A surviving mutation is a test that passes on broken code. Kill it with a test
  or remove the test.
- Look for duplication where a form repeats three times: parsers, codegen,
  fixtures.

A measurement does not change code. Each finding becomes a tracker task. Debt
that blocks the current task is bypassed narrowly and named in the report.

## Commit

One task means one commit. The user commits after review. The agent does not
commit and does not push.

## Report

After a task, write one line: what changed and how to check it.

## Ambiguity

Stop and ask. Ask a question instead of choosing for the user. Ask about spec
ambiguity and any open decision. An external dependency is not a question. Name
it before the change, state what code it removes and why the popular library is
better than local code, then use it if it helps.

A new crate is a boundary, not just another folder. Create one when a boundary
is needed, not when a certain number of consumers exists. A GUI framework has
many consumers, so do not wait for a second one. A boundary is needed when a
module has its own dependencies, when without it code falls into the wrong layer
or closes a cycle, or when it must compile on its own. Name the crate before the
change: its name, its responsibility, what moves, and what stays. Create it
without asking for permission. The move is its own task and its own commit. Do
not mix behavior into that commit.

## Tracker

The `BRUL` tracker is the source of truth. A task states what to do and why. A
reader must understand it on its own. Open a task for work that must be
finished, not for a one-line detail. Close a task when the work is finished and
the checks are green.

A spec change that alters the contract, such as making `;` mandatory, is a
separate task. It needs no permission. If a spec blocks DX, Rust-way, or
performance, change the spec and state what changed and why in the report. A
change that only brings text in line with an already accepted contract does not
open a task. State it in the report or in a comment on the task that introduced
the contract.
