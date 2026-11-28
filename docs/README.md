# BRUL documentation

The tree is split by reader. There are no shared pages. A repeated fact is an
error. Use a link instead of a second copy.

## Sections

| Path                        | Reader                                          | Publication                               |
| --------------------------- | ----------------------------------------------- | ----------------------------------------- |
| [dev/](dev/architecture.md) | Developers: design decisions and file locations | No                                        |
| [lang/](lang/README.md)     | DSL canon                                       | Included in user docs with mdBook include |

- [dev/architecture.md](dev/architecture.md): structure, crates, and pipeline.
- [dev/product.md](dev/product.md): goals, framework requirements, and maturity
  criteria.
- [dev/workflow.md](dev/workflow.md): tasks, budget, reports, and tracker.
- [dev/benchmarks.md](dev/benchmarks.md): measurement snapshots and comparison
  rules.

## Rules

1. **One fact, one home.** Use a link instead of a second copy.
2. **Do not describe what code can prove.** Grammar, defaults, and signatures
   live in code and tests. The docs point to them.
3. **Docs have a lifetime.** A design change and its code change land in the
   same commit. Git keeps history. There is no archive of decisions. Remove
   deferred items when the question is closed. Reduce a section that is no
   longer complex to a pointer line.
4. **No status text.** "Phase closed" and "tests are red" belong in `git log`
   and CI.
5. **Do not rewrite user docs by hand.** Include the canon and generate text
   from rustdoc.
