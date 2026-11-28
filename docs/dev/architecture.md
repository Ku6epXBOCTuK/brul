# Architecture

BRUL is a framework for native desktop applications in Rust. It provides a
declarative UI in its own DSLs. Markup, styles, and script live in one `.brul`
file. The compiler turns them into widgets on winit and wgpu. BRUL does not
render HTML. A WebView attaches as a platform view.

## Crates

```sh
brul                   facade: `Ui`, startup, bindings, and built-in input policies (`src/policy`)
brul-codegen           DSL loading and codegen: resolve/import, styles, Rust emission
brul-event             dictionary of raw and semantic events
brul-gui               primitives, text, and widgets
brul-layout            geometry: layout types and the taffy adapter
brul-reactive          minimal core: signals/memo/effect
brul-script            AST interpreter: values, environment, execution on a component
brul-style             style model: properties, values, rules, and cascade
brul-syntax            SFC splitter, lexer, and DSL parsers that build the AST
brul-window            winit+wgpu: window, event loop, translation of input into raw events
examples/hello         example: a window, a host command, and kit buttons
```

A crate is a boundary. Create one when a boundary is needed, not when a second
consumer appears. Name the crate, its responsibility, and what moves before the
change. Do not ask for permission. The move is a separate task. An external
dependency is also decided by benefit, not by permission.

## Pipeline

```sh
app.brul ─ SFC splitter ─> markup | style | script
  markup ─ parser ─> AST ─ codegen ─> Rust module ─> widgets, bindings, structures
  style  ─ parser ─> snapshots (compile-time constants)
  script ─ parser ─> AST ─ emission into the same module, execution on a component
signals ─> effect ─> bindings and structures ─> brul-gui ─> frame
```

- Typed state lives in the application, not in widgets.
- Interactive input state, that is caret, selection, preedit, scroll, and caret
  geometry, lives in the leaf widget. The application has no UTF-8 boundaries
  and no IME geometry to store it instead.
- Background tasks use tokio. Results reach the UI through a channel, because
  the reactive graph is single-threaded.
- Several windows each have their own tree root. Shared signals go through the
  application owner.

## Decisions

This page holds only the current state. A decision changes in the same commit as
its code. Git keeps the edit history and earlier options.

### Layout and styles

The `Container` handles geometry only. It takes flow direction from
`flex-direction` in the style. Text is encapsulated in `Text` and a separate
text engine.

A text block is `Text` with classes. Inside it are semantic roles: `Accent`,
`Term`, and `DropCap`. Styles, not tags, define the look of a role.

There is one box model: border-box. A set size is final and includes padding and
border. Offsets are additive. There is no `margin`.

Styles are parsed once into immutable snapshots. A state switches the snapshot
pointer. Therefore only visual properties change with state, not geometry. The
exception is font weight. A `font-weight` change reshapes `Text` inside its box
without recalculating container layout. See
[the text metrics rules](../lang/style.md#text-metrics).

Instead of `float`, use exclusion zones. The text renderer shortens lines that
cross objects.

Built-in styles form a separate layer. `@apply` directives in
`builtin/widgets.brul` bind classes to primitives and events. The directives sit
next to the rules. The loader parses the file with the entry-point palette, so
built-in rules can also depend on the theme. The loader applies the bindings to
markup before codegen. It composes layer tables at the code output boundary. The
generator knows nothing about built-in classes. It emits only an ordered list of
indices.

`brul-layout` computes layout. `taffy` provides flexbox with grid and wrapping.
BRUL public types stay its own. The crate converts `LayoutStyle` to
`taffy::Style`. CSS length properties `flex-grow`, `flex-shrink`, `flex-basis`,
and `gap` pass through. Absolute `x` and `y` become `position: absolute`. Grid
and wrapping switch on when the property appears in markup. `taffy` rounds per
node. Whole physical pixels leave the crate.

Sizes are auto, px, or a percent of the parent content box. `taffy` resolves a
percent against the parent size it already has. It does not contribute to
min-content, unlike CSS. `justify-content` moves items on the main axis.
`align-items` moves them on the cross axis. `space-between` and `stretch` are
the extreme values.

`WidgetTree` walks nodes and writes boxes straight into slots. `Text` measures
with `TextEngine` at content width. `Image` takes the texture size. `Container`
becomes a flex node with direction from `flex-direction`.

A snapshot is a flat list in walk order: geometry, resolved style, glyphs, and
texture. Rendering does not walk widgets.

Hit testing uses rectangles. Raw `Press` and `Release` follow the path from leaf
to root. The `pointer` policy emits a semantic `click` on each element of the
path when release happens over the original widget. A disabled widget and its
subtree receive no events.

A `Layer` is a node outside the flow and sized to the window. The snapshot and
hit test walk layer children after normal children, in reverse declaration
order. Therefore the layer draws over content and receives input first. The
layer does not capture input itself. Its child does.

The tree's `focusables()` applies the modal area. The `keyboard`, `pointer`, and
`scroll` policies call `enter_modal_scope()`, send `dismiss` on Escape, and
limit the wheel to the area. Focus is saved and restored when the layer hides.

`Frame` draws the snapshot. It is an RGBA8 premultiplied CPU frame. A full frame
clears everything and walks the snapshot. A partial frame takes the bounding box
of dirty nodes, clears it, and redraws intersecting nodes in walk order. The
frame goes to the wgpu surface as one texture. There is no separate GPU pipeline
for primitives and text. A size, DPI, or layout change produces a full frame.

The snapshot is built once. A state, class, or hover change edits only the dirty
node records in place. The vector and geometry stay as they are. A structural
change, layout, or focus change rebuilds it, because children and clips change.
An index finds a node by id, not by a linear walk.

Why: a developer always knows the final size of an element. A frame costs what
changed, not the size of the tree. Component styles do not leak into neighbors.

Rejected: the HTML model, where an element is both a container and holds text.
Also rejected: a CSS cascade with specificity on every mouse move.

Norm: [style.md](../lang/style.md), [markup.md](../lang/markup.md).

### Text

`cosmic-text` handles shaping, fallback, and rasterization. It covers system
fonts, Cyrillic, and emoji without a custom shaper. The engine returns a
1024×1024 RGBA8 atlas and a glyph list. Cells sit in shelves with a 1 px gap and
are reused by a rasterization key that includes size and DPI.

A style is set in logical pixels. `scale` converts it to physical pixels. Whole
physical coordinates leave the engine. Subpixel antialiasing is reduced to a
mask. The renderer sets the color of a normal glyph. A color emoji sits in the
atlas as RGBA.

Why: `cosmic-text` already solves fallback and emoji. The atlas separates
shaping from GPU work. The MVP atlas does not grow. When there is no space,
`layout` returns an error.

Rejected: a custom shaper and custom font loading.

The semantics are the contract of `brul-gui`: crate-doc and tests.

### Primitives

Geometry uses whole physical pixels. `Rect` stores x and y, width and height,
radius, fill, and border. The border sits inside the set size, as in border-box.
The radius is at most half the shorter side. Color is RGBA8.

One SDF formula describes rounding, fill, and border. A sampler checks it at the
pixel center. The CPU frame uses the same `Rect::sample`.

An image is a `Texture`. PNG is decoded to RGBA8. Tint is stored next to the
pixels and multiplies RGBA at sample time. A theme change does not re-encode the
image. Only PNG is decoded. Other formats come later.

Why: whole coordinates match layout and do not shake at boundaries. One formula
keeps CPU checks and rendering in agreement. A tint parameter avoids several
copies of the same pixels.

### Window

A window uses winit 0.31-beta.4 with an exact version pin and wgpu 30. Config is
in logical pixels. winit returns the physical surface size.

`frameless` makes the window borderless, transparent when `transparent` is set,
and fixed in size. Without it the window is decorated and resizable. Header drag
works on any window. System close closes the window.

winit is hidden in `brul-window`. The rest of the code sees `WindowConfig`,
`WindowKey`, `WindowSpec`, and `WindowError`.

One `EventLoop` holds several windows. `run_windows` takes a start key, a window
content factory, and an event handler by key. Each window has its own surface,
renderer, text engine, timers, and cursor. The loop, the wgpu device, and the
system theme stay shared.

`Open(key)` and `Close` commands from the handler create and close windows.
Closing the last window ends the loop. `run_tree` and `run_tree_themed` are
single-window wrappers.

The surface is created in `can_create_surfaces`. The alpha mode comes from
surface capabilities in this order: PreMultiplied, PostMultiplied, Inherit,
Auto. Frame latency is set to 1.

`run` clears the background. `run_tree` draws the snapshot and hands the
finished CPU frame to the surface. There is no separate GPU renderer for
primitives.

The frame texture goes to the surface only when there is something to show:
non-empty dirty after a rebuild, first display, a size change, or a lost frame.
Waking the loop without changes copies no RGBA buffer and performs no blit.

Each window creates its own `wgpu::Instance` and device. A shared context is a
separate task.

A platform view is a snapshot node with its own `PlatformViewId` and source.
BRUL neither draws its content nor owns its pixels. After layout, `brul-window`
syncs bounds and visibility, passes focus to the child window, and removes input
and IME from the tree while that window has focus. WndProc interception and
manual `WM_*` routing are not used. The child window receives pointer and
keyboard input itself.

The first iteration is an isolated child window above BRUL. It is always above
BRUL pixels. It cannot be clipped. Rounding, effects, and overlays do not affect
it. Software and layered modes and Wayland are not supported.

`WebView` is the first platform view backend. `brul-webview` uses wry to create
a child window with `build_as_child`. It does not pull winit into the runtime
and does not write WebView2 COM by hand. The factory is set in
`run_generated_with_platform`. Bounds from the snapshot are already physical.

IPC messages go through `PlatformMessageHandler` to `EventSink` and
`Handle::widget`, that is to the `message` event of the element. The first
iteration targets Windows and an opaque GPU canvas.

Why the beta: version 0.31 separates surface creation from `resumed`. It gives
one pointer path for mouse, pen, and touch. It fixes Windows bugs with DPI, IME,
and drag. The version is pinned, so an API change cannot arrive with
`cargo update`.

Rejected: `softbuffer`, which has no per-pixel transparency. Also rejected:
Windows-only DirectComposition, which drops other platforms.

### Build and windows

The application `build.rs` makes one call:
`brul::build::app("src/window.brul")`. It writes `view.rs`, a `bc_<stem>` module
for each included `.brul` file, and its `<script>` source as a sibling `.script`
file into `OUT_DIR`. The build includes them in the binary.

Kit components such as `brul:button` come from a table embedded in codegen when
BRUL is built. The application has no `kit/` copy.

`main.rs` starts generated code through `brul::run_generated`. Initial values
arrive as seed objects. Window actions arrive as host commands marked with
`#[command]`.

A command argument is any type with
`TryFrom<&brul::Value, Error = brul::Error>`. `brul-script` implements it for
scalars: `&str`, `String`, `f64`, `bool`, and `usize`. Enums and structs
implement the trait. `brul::Fields` reads object fields. A conversion error
reaches the DSL as `Error::Argument` with the command name.

`run_generated` creates one `Component` and one `Ui` per open window. Windows
share script and signals but have their own trees, handlers, and command queues.
A window event is attributed to that window.

`openWindow(name)` opens the declaration by name. An already open window moves
to the front. `exit()` and `closeWindow()` close the current window. Closing the
last window ends the loop. The theme is shared. `setTheme` rebuilds all open
windows and remembers the choice for future windows.

The theme is set at startup. `Dark` and `Light` pass a theme name into `view`.
The system theme is `None`. Then the window follows winit `ColorScheme` and
`ThemeChanged` and rebuilds the tree when the scheme changes.

`run_generated_with` differs from `run_generated` only in its last argument.
Before the loop starts, it gives the caller a `brul::Handle`. A background
thread uses it to write a DSL variable (`handle.set("live", true)`) or call a
script function (`handle.call("hide")`, and with arguments
`handle.call_with("show", vec!["main".into()])`). The string path stays
low-level.

`Handle` is cloneable and outlives the thread. Messages are applied in the event
loop after wake-up and redraw windows. After the loop exits, `wake` does
nothing, so a hanging thread does not panic.

Codegen prints `generated::Script` next to the module. It wraps `Handle` with
one method per script function and its arguments, for example
`script.say_hello()` and `script.bump(2)`. A method sends a typed message. A
typo in a name does not compile through the wrapper. Dynamic values pass through
`brul::Dyn`. Conversion to the runtime representation stays inside `brul`.

`build.rs` reads `#[command]` signatures from Rust sources.
`script::validate_build` checks call names and types at build time. A typo or a
wrong type stops `cargo build` with a path and line number from the `.brul`
file. The generated call goes through the typed marker `HostCommand`.
`Host::call(&str, &[Value])` remains the low-level path for dynamic calls.

Why: a theme change tests the whole path before rewriting DSL. Selection, hover
and press from the `pointer` policy, checkboxes, and startup all go through the
same widget event path.

### Updates without VDOM

An "element ↔ expression" binding is known at compile time. Codegen emits one
direct update function per binding. A signal wakes only the affected functions.
Rendering redraws only dirty widgets.

Effects run once per frame, before drawing, after a batch of events.

A property is a signal. Reading it in an expression subscribes the binding to
it. Writing it wakes only those readers.

Environment variables and host functions are not signals. Reading them
subscribes to one shared `external` signal. A variable write and `touch()` move
that signal.

Generated closures compute binding values. The module uses
`brul::Dyn::{binary, unary, template}`, `component.call_dynamic`, and property
reads through `component.read_id`. There is no baked-in AST.

`if` and `each` structures compile into `value` and `rebuild` functions. An
effect watches the condition and queues the structure. `Ui::settle` replaces the
subtree after `flush`.

`rebuild` receives rows from the previous pass. `RowKey` identifies an `each`
row by an explicit key or by position. `RowMatcher` finds a pair by key hash and
checks the element with equality in that bucket. A matching row is returned
without a build. `settle` takes its widgets from the old tree. Only changed rows
are rebuilt.

`settle` finds a structure by parent hash. The tree is borrowed once per pass,
not once per binding and handler.

An event finds its handler through a "widget and event kind" index, not by
walking a list. The list changes only during a rebuild and at its tail. The
index is rebuilt there too.

Raw pointer events carry coordinates and modifiers for policies. Script is not
called from them, so no argument object is allocated. An argument object is
built only for semantic `key` and `input` events.

Builds use an explicit context. `view` and `rebuild` receive `&mut brul::Build`.
It holds the current component slots, services (`states`, `event_sink`,
`clipboard`), and the list of registered structures. A nested component and a
slot view receive the same context. `Ui` takes the call result from it through
`finish()`.

Capability hooks work with `&brul::CapabilityContext`: services, target, state,
and component.

Why: a node has an address for its whole lifetime, with a handle and a binding
known to the compiler. Updating a value means writing a node property, not
rebuilding a subtree. The unit of change matches the unit of invalidation. A
signal wakes exactly the binding functions that read it. A frame costs changes,
not the tree. State lives outside widgets, so "new data, old widget" is
impossible by construction. Subtree death points are known to the compiler, and
an effect scope ends with its widgets without a runtime diff.

Generated code uses concrete types and direct calls, without boxing or generic
builders. Widgets registered in Rust are the same `View` functions. Codegen
checks their props and events against the declaration. `Built` is assembled from
existing primitives, without trait objects.

Rejected: a virtual tree and diffing. Also rejected: imperative builders in the
style of gpui, where an element is a temporary value with no address. There an
update requires a subtree rebuild and an engine comparison, and invalidation is
coarser than a signal.

Consequence: a binding cannot be created at runtime from a string. Application
state lives outside widgets, and a widget only reflects it. The exception is
temporary input state: caret, selection, preedit, and scroll belong to the leaf
widget together with IME geometry. The field value stays application state and
syncs through `value:` and `on input`.

### Reactive core

BRUL has its own minimal core with lazy recomputation and push-pull, as in
[alien-signals](https://github.com/stackblitz-labs/alien-signals). The public
API is `Signal`, `Memo`, `Effect`, `EffectScope`, `batch`, `untrack`, and
`flush`.

The implementation is an arena of `Vec<Node>` with `u32` indices. Subscription
lists are `Vec`s. An explicit stack handles marking. The stack and the effect
queue are buffers of the arena itself, so pushing a signal with no new
subscribers allocates nothing. Values and closures live in typed
`Rc<RefCell<...>>` cells.

A stopped effect slot goes to the free list and is reused by the next node.
Therefore `if` and `each` rebuilds and theme changes do not grow the arena.

There is no version comparison and no `Watched` flag. A write eagerly marks the
whole graph below it. The graph is single-threaded and lives on the UI thread.
Background tokio tasks send updates through a channel.

Ownership: `EffectScope` owns effects and `on_cleanup` callbacks. Scopes are
hierarchical. A `child` refers to its parent. `dispose` ends children, is
idempotent, and is called from `Drop`. Therefore no cleanup can be missed.

`Ui` has one root scope. Each binding, prop, and structure gets a child scope.
An effect lives while its widgets live and ends with them. Each adopted `Ui`
component also gets a child scope and a fresh `TimerOwner`. The old owner cannot
address the new one after a rebuild.

A scope does not hold signals and memos. The last `Rc` owner frees them.

An `every` timer goes to the window host as a command. It is registered with the
component scope. When the component disappears, the scope sends `Cancel` for all
its `TimerId`s.

Shared application state, such as theme and a second window, is created before
the window. A prop receives the computed value.

Why: subtree destruction points are known before compilation: window close, `if`
branch change, `each` row removal, component removal. RAII on a scope gives the
same guarantee as RAII on every node and keeps handles `Copy`. A value is not
cloned into every closure.

Rejected: RAII on handles. A parent writes a prop and a child reads it, so a
signal has two owners. Drop order would become a source of silently frozen
widgets.

Deduplication sits at the write boundary, not in the graph. A widget setter does
not mark a node dirty on an equal value. The host does not write an unchanged
poll result into a signal.

The core does not compare versions. `notify` queues an effect before memo
recomputation, so the fact that a value is unchanged is already lost by `flush`.
A `changed` flag or versions cost more than they save.

Why: expensive operations are `Text` reshaping and redraw, not the effect walk
itself. Comparing at the boundary is cheaper and sufficient.

Why: a foreign core pulls in owners, contexts, and `Arc<RwLock>`. That is
overhead for features BRUL does not use. `alien-signals` and `reactive_graph`
are sources of good practice, not an API template.

Rejected: `reactive_graph` from Leptos. Also rejected: `sycamore-reactive`,
which uses lifetimes tied to another framework and has unstable releases. Also
rejected: `futures-signals`, which is push-only and whose future effects do not
fit a frame flush. Also rejected: Dioxus signals, which are not a standalone
crate.

The semantics are the contract of `brul-reactive`: crate-doc and doc tests, not
prose here. List diff updates in the style of `SignalVec` come after MVP.

### Parsers and tooling

The SFC splitter, which divides `.brul` into blocks, uses pest. The three
languages use hand-written recursive descent over the EBNF in `lang/*.ebnf`.
Errors carry positions. Recovery is controlled. The code is easy to teach.

The spec is the contract. Tests extract its examples and use them as fixtures.
`just generate-rules` generates production lists from EBNF. A test checks them
against the files. Negative cases are marked in the spec. Therefore the parser
cannot silently drift from the spec in either direction.

Why: pest is good for the regular structure of blocks. The languages need error
recovery and exact positions.

Deferred: ready-made LSP, formatter, and linter come after the languages
stabilize in practice. Their foundation is not deferred. The AST is built
without execution. Nodes have positions. The spec is the contract.

The format for codegen and LSP is not chosen: ungrammar, tree-sitter, or ANTLR.
The criterion is that one description builds both the parser and the LSP.

Norm: [lang/](../lang/README.md).

### DSL limits

Markup uses brackets, not tags. Structure is separate from logic and
presentation. The universal container is `Container`. Leaf widgets are `Text`
and `Image`.

A handler is a named function only. Inline bodies are not allowed. Repetition is
`each`. A condition is `if`. An item key goes into the row structure. An `each`
rebuild reuses widgets by key.

Components receive children through `@slot`, including at the root. Such a
component is a fragment with no layout node. Its children go into the caller's
layout. A class on a component call is not accepted.

Script is a small JS-like language. It has no classes, no `this`, no `new`, no
ASI, and no automatic casts. A change of model happens by assigning a new value.

Styles use CSS-like syntax without cascade, inheritance, or specificity.
`PascalCase` names widgets and components. `kebab-case` names classes and theme
keys. `camelCase` names properties and events.

Why: a familiar syntax with little implementation work. A type or structure
error is visible in place, not deep inside an expression.

Rejected: HTML tags, which mix meaning and presentation. Also rejected: a
TypeScript subset, which adds work for component handlers.

Norm: [lang/](../lang/README.md).

### MVP boundaries

The goal is the framework. Its requirements and maturity criteria are in
[product.md](product.md).

The MVP platform is Windows. The architecture does not close other platforms.
There are no dates. The order of steps matters more.

These items do not block MVP: a ready-made LSP, formatter, and linter (their
foundation is a requirement, see product.md), animations, SVG, scroll polishing,
and a second window.

Why: daily use is the only check that a framework works and not just looks good
in examples.

Deferred: all items from the "does not block" list, after MVP.
