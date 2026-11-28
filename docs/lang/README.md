# DSL canon

Four languages describe the UI. The spec is the contract. Tests parse its
examples, so the examples also act as fixtures.

- [markup.md](markup.md): markup, that is UI structure, components, and
  bindings.
- [style.md](style.md): styles, that is classes, states, and themes.
- [script.md](script.md): script, that is handler functions and expressions.
- [use.md](use.md): imports, that is the `<use>` block, relative paths, and kits
  through `brul:name`.

## Location

The canon lives here: `markup.ebnf`, `style.ebnf`, `script.ebnf`, `use.ebnf`.
Markdown specs include these files through mdBook include. No copies appear. The
parser moves the files to `crates/brul-syntax/lang/` without edits.

## Shared conventions

- Expressions in markup and script use one expression language. The grammar
  lives in [script.ebnf](script.ebnf). Do not repeat it in
  [markup.ebnf](markup.ebnf).
- A component import is a separate `<use>` block in a file. The syntax is `Use`.
  Markup can use the name.
- The universal container is `Container`.
- Versioning: a draft can change freely. A spec is stable when the parser passes
  all spec examples and the examples have not changed since the first working
  integration. After a spec is stable, a breaking change needs a new spec
  section with a reason.

## EBNF dialect

- `=` and `;` define a rule. `|` marks an alternative. A space marks
  concatenation.
- `[ … ]` marks an optional part. `{ … }` marks repetition. `( … )` marks a
  group.
- `"…"` marks a terminal string. `? … ?` marks a terminal character class.
- `(* … *)` marks a comment.
- A rule from another file uses `Script.Expr`. A rule without a prefix belongs
  to the current file.
- Terminals are declared at the end of a file. Identical names in different
  files are not linked.

The parser strategy is in
[architecture.md, "Parsers and tooling"](../dev/architecture.md#parsers-and-tooling).
