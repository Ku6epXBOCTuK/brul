# Import language

Status: stable. The canon of the language is this spec. Tests check the import
parser against it.

The principles are in
[architecture.md, "Decisions"](../dev/architecture.md#decisions).

## Concept

A component is reused when its file is declared in a `<use>` block. An import
declares a file. It is not a script instruction. Markup can use the name as a
component. A resource is declared in the same block. Markup can use its name as
an `Image` source.

The `<use>` block is part of the SFC structure. See
[markup.md, "Files and imports"](markup.md#files-and-imports).

## Grammar (EBNF)

```ebnf
{{#include use.ebnf}}
```

## Semantics

- A component import is `import Name from "./path";`. No other import syntax
  exists. The path is relative to the file that holds the `<use>` block. The
  `.brul` extension is added when you omit it.
- A kit import is `import Button from "brul:button";`. After `brul:` is a file
  name in `kit/<name>.brul` from the BRUL repository, not a path. The build
  embeds the source in `brul-codegen`. A name with a slash or `..` is not
  allowed. An unknown name is a build error with a list of available names.
- A kit has no relative imports. Its files import each other through `brul:` as
  well. A resource in a kit file is declared as in an application. Its path
  starts at the `kit/` directory: `resource Check from "./icons/check.svg";`.
  The build embeds the bytes too.
- A resource is `resource Name from "./path.png";`. Its path is also relative to
  the file, but the extension is part of the path and is never added.
- A resource is PNG or SVG. The build embeds the file bytes in the binary. The
  file must exist and start with a PNG signature or SVG markup. Otherwise the
  build fails. A duplicate resource name in one block also fails the build.
- SVG is rasterized on first display at the size that the `Image` gets on screen
  in physical pixels. The cache key is size and color. `tint` replaces
  `currentColor`. A file without `currentColor` uses its own colors.
- Markup in this file can use the resource name as an `Image` source, for
  example `Image (icon) { check }`. The name is not a component and not a script
  value.
- The loader parses each component it finds. Their own `<use>` blocks form a
  chain. A cycle is a load error with the chain path.
