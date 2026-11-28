# Script language

Status: stable. The canon of the language is this spec. Tests check the parser
and the generator against it. The examples at the end are test fixtures.

The principles are in
[architecture.md, "DSL limits"](../dev/architecture.md#dsl-limits).

## Concept

```script
let remembered: Object = loadRemembered();
let domain: String = currentUrl();
if (remembered[domain]) {
  launch(remembered[domain]);
}
```

Script uses static typing. A type is given in an annotation. There is no type
inference.

Script compiles into Rust functions and runs on the UI thread. Script works with
**component properties**. A read reads a signal. An assignment writes a signal.
UI updates are point updates through brul-reactive.

## Values and types

The [type table](types.md) is generated from `brul-lang-schema`. It holds names,
literals, and notes.

Compound types are **arrays** `[a, b, c]` (`Array<T>`) and **objects**
`{ name: "Chrome", flags: "" }` (`Object`). Objects are needed for `each` models
and configurations. Access is `item.name`, `arr[0]`, and `len(arr)`. See the
built-in functions. An empty array takes its type from the annotation:
`let items: Array<String> = [];`.

`null` has type `Null`. It is not compatible with other types, including
`Image`.

**`Image`** is a path to PNG or SVG. The value is created by the application as
a seed. Script only passes it to markup. An empty image has its own literal
`empty`: `let icon: Image = empty;`. `null` is not compatible with `Image`.

**`Void`** is only the return type of a function without a value. There are no
`Void` variables and no `Void` parameters.

`Object` fields have type `Any`. An access such as `ev.code` is not checked
statically. Writing it to a prop or passing it to a typed function needs an
explicit conversion: `String(ev.code)`.

A type is declared everywhere: in `let` and `const`, in parameters, and in
function returns. A component property is typed in markup
(`prop checked: Bool;`). The script annotation must match the prop type.

The generated module uses concrete types. `Value`, `read_id`, and `assign_id` do
not remain there. Dynamic values pass through the `brul::Dyn` facade.

## Lexicon

- A comment runs from `//` to the end of the line.
- Identifiers use `camelCase`.
- A semicolon is **required** at the end of a statement. There is no ASI, that
  is no automatic insertion.
- Keywords are `let`, `const`, `if`, `else`, `function`, `return`, `true`,
  `false`, `null`, and `empty`.

## Grammar (EBNF)

Expressions are shared with markup. The entry point from markup is `Expr`.
Precedence runs from weak to strong.

```ebnf
{{#include script.ebnf}}
```

- `+` adds numbers. For strings it concatenates. Mixed types are an **error**.
  Use explicit conversions through built-in functions. There is no JavaScript
  auto-casting.
- Assignment is valid only on a component property (`checked = ...`), a variable
  (`let` or `const`), or an array or object element. The value type must match
  the assignment target type.
- Functions exist only at the top level of a script block. Nesting is not
  allowed in MVP.
- A type annotation is required in every `let` and `const`, every parameter, and
  every function return. There is no type inference. `let x = 1;`,
  `function f(x) {}`, and `function f(x: Number) {}` without `: Void` are build
  errors with a position.
- An undefined variable, property, or function and a type mismatch are **build
  errors** with a position. All names and types are resolved statically:
  component props, local `let` names, functions, built-in functions, and host
  functions.
- `return value;` ends a function and returns a value of the annotated type.
  `return;` is valid only in `Void` and `Null`. `return` outside a function, a
  wrong return type, and returning a value from `Void` are build errors.

## Execution semantics

- **Context:** a component script block runs once when the component is created.
  That initializes `let` variables and initial values. Handlers and functions
  run on an event or a call. They can access all names of the component.
- **Variables:** `let` allows reassignment. `const` does not. A second write to
  a `const` is a build error. `const` requires an initializer.
  `const x: Number;` is a build error. `let x: Number;` without an initializer
  is valid only at the top level. It declares an application seed whose value
  must have type `Number`. Inside a function such a declaration is a build
  error.
- **Functions:** only at the top level. MVP has no closures. The body runs in
  the component context. Parameters and return values are typed by annotations.
  A wrong argument type at a call is a build error. `return` gives a value of
  the annotated type. `Void` ends with `return;`. Recursion is allowed. The
  return annotation makes the signature known before the body walk.
- Markup can use functions as handlers. Calling functions from markup
  _expressions_ is outside MVP. Only built-in functions work there.
- **Host function signatures:** they are set on the Rust side in `#[command]`.
  For example `fn foo_bar(x: String) -> bool` is available as
  `fooBar(x: String): Bool`.
- `build.rs` reads these signatures from Rust sources. The generator checks
  calls and prints an error with a `.brul` path and line number. Generated code
  calls a command through a typed marker. It does not use
  `Host::call(&str, &[Value])`.
- **Properties are signals:** a read inside a markup expression creates a
  dependency, that is brul-reactive tracking. An assignment writes a signal and
  updates the UI at that point. A read inside a handler has no tracking.
- **Write conversion:** only exact type matches are valid. A mismatch is a build
  error with a position. Getting a `Number` from a `String` needs an explicit
  `Number(x)`.
- **Runtime errors:** an index outside an array or division by zero interrupts
  the handler. The error is logged with a position. The application does not
  crash.
- **Thread:** all script runs on the UI thread. Long operations run on the Rust
  side with tokio. Script receives the result through a signal. Promises come in
  v3+.
- **Timers:** `every(ms, fn)` calls a function every `ms` milliseconds on the UI
  thread in the component context. The first call is after `ms`. `after(ms, fn)`
  calls it once. Both return a `Timer` handle. `cancel(timer)` accepts it.
  Cancel before the first call suppresses it fully. `fn` is the name of a
  top-level function with signature `(): Void`, as for handlers. Functions as
  values are outside MVP.
- A timer lives in the scope of the component that created it. When the
  component disappears after a tree rebuild or window close, the timer is
  cancelled. An old handle does not fire in a new component, even if it got the
  same `TimerId`.

## Built-in functions (MVP)

The [built-in function table](builtins.md) is generated from `brul-lang-schema`.

`Any`, `Function`, and `Timer` are pseudo-types used only in the signature
table. They are not valid in script annotations.

The set is minimal. The host extends it without changing the language.

## Construct status table

| Construct                                                                                        | Status                                   |
| ------------------------------------------------------------------------------------------------ | ---------------------------------------- |
| `let`/`const`, literals, operators, ternary, calls, `if/else`, arrays, objects, string templates | **MVP**                                  |
| Type annotations in `let`/`const`, parameters, and returns                                       | **MVP**                                  |
| Type inference and optional annotations                                                          | never                                    |
| Top-level function declarations (`function`)                                                     | **MVP**                                  |
| `for`/`while` in script                                                                          | v3+; UI repetition uses `each` in markup |
| Closures, nested functions, functions as values                                                  | v3+                                      |
| `try`/`catch`                                                                                    | v3+                                      |
| `async`/`await`                                                                                  | v3+; promises                            |
| Classes, `this`, prototypes, `new`                                                               | never                                    |
| `var`, ASI, implicit casts, `==` with type conversion (`==` is strict)                           | never                                    |
| Array methods (`map`/`filter`/...)                                                               | v3+ as needed                            |
| Export modules (`export`)                                                                        | v3+; now one component per file          |

Details fixed as norm:

- Change a model by assigning a new array to the property (`browsers = [...]`).
  There are no `push` or `pop` methods.
- `{` in statement position starts a block, as in JavaScript. An object literal
  in statement position is a compile error.
- Indexed writes `arr[i] = x` are allowed.
- Identifiers use only `camelCase`. A hyphen is not possible because it
  conflicts with subtraction.

Configuration data is read with a string index and no conversion:
`settings["always-ask"]`, `browser["profile-dir"]`.

## Test examples (cover the grammar)

### 1. Script of the picker component

The host function signatures in the example are `loadBrowsers(): Array<Object>`,
`loadRemembered(): Object`, and `openWith(path: String, flags: String): Void`.

Examples with annotations are marked `ignore`. The parser will check them when
it supports type annotations.

```script,ignore
let browsers: Array<Object> = loadBrowsers();
let remembered: Object = loadRemembered();
let currentUrl: String = "";
let selectedIndex: Number = 0;

if (len(browsers) > 0) {
  selectedIndex = 0;
}

// вызывается из разметки: on clicked(i): launch
function launch(i: Number): Void {
  let browser: Object = browsers[i];
  openWith(browser.path, browser.flags);
}
```

### 2. Handlers (named functions only, no inline bodies)

```script,ignore
function handleKey(ev: Object): Void {
  if (ev.code == "Escape") {
    exit();
  }
}
```

The markup attachment in the window is `on key: handleKey`. Script checks the
key code. There is no special grammar for individual keys.

For shortcuts with letters, compare `ev.physical`, for example `"KeyQ"`. That
code does not depend on case or keyboard layout. `ev.code` can be `"q"` or
`"Q"`.

A window handler receives `key` even when an element has focus. The key goes to
the element first and then to the window. Keys consumed by a policy do not reach
`on key`. `Tab` changes focus. `Enter` and Space on a button produce `click`. In
a modal layer, `Escape` reaches the layer as `dismiss`.

### 3. Markup expressions (the same expressions, another consumer)

```script,ignore
Text (url-text) { currentUrl }
Text (count) { `Выбрано: ${selectedIndex + 1} из ${len(browsers)}` }
Image (check-icon, visible: selected) { checkSvg }
Container (item-panel, selected: i == selectedIndex) { }
```

### 4. Timer for status refresh

```script,ignore
function refresh(): Void {
  reloadServices();
}

every(3000, refresh);
```

`reloadServices(): Void` is a host function of the service panel. It starts
status reads on the Rust side. The result arrives in the `services` property.

### 5. Call without declarations (the current parser handles it)

```script
exit();
```
