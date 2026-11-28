# Markup language

Status: stable. The canon of the language is this spec. Tests check the parser
and codegen against it. The examples at the end are test fixtures.

The principles are in
[architecture.md, "Decisions"](../dev/architecture.md#decisions).

## Concept

Markup is a bracket DSL instead of HTML tags. An element is a name, parentheses
with classes and parameters, and braces with content:

```brul
Container (item-panel, selected: selected, on click: panelClicked) {
    Text (text-main) { name }
}
```

- The name is a built-in widget or a component.
- Inside the parentheses, a bare identifier is a **style class**.
  `name: expression` is a **parameter**. `on event: function` is a **handler**,
  by reference only, with no inline body.
- Inside the braces is structure: child elements, `@if`, and `@each`. A leaf
  widget such as `Text` has one content expression. The `component` block is a
  known exception. It declares `prop` and `event` next to the structure.

Appearance comes from classes in styles ([style.md](style.md)), not from inline
parameters. Parentheses hold only behavior and data.

## Lexicon

- A comment runs from `//` to the end of the line. `/* */` is deferred.
- Built-in widget and component names use `PascalCase`.
- Classes use `kebab-case`. Properties and events use `camelCase`. Theme keys
  use `kebab-case`.
- Spaces and line breaks are not significant, except inside string literal
  bodies.
- Keywords are `component`, `window`, `prop`, `event`, `on`, `in`, `import`, and
  `from`. Prop types are `Bool`, `Number`, `String`, and `Image`.
- Block constructs start with `@`: `@if`, `@else`, `@each`, and `@slot`. The
  marker separates them from widget and component names.
- A keyword cannot be an identifier. `on` inside parentheses starts a handler. A
  class or parameter cannot have that name.

## Grammar (EBNF)

`Expr` expressions are shared with script. See
[script.md, "Grammar"](script.md).

```ebnf
{{#include markup.ebnf}}
```

The exception is **leaf widgets** (`Text`, `TextInput`, and `Image` in MVP).
Their block holds one expression, that is content, not a list of children. The
rule is semantic, not syntactic. The parser knows the widget class.

```brul
Text (text-main) { name }
Text (badge-text) { "По умолчанию" }
```

## Semantics

### Window

`window` is the root component of a window. Window parameters are not styles.
They are window system attributes. `frameless` and `transparent` are Boolean
flags in parameters. Both are `false` by default. The window is then normal,
decorated, and opaque.

`visible: false` creates a hidden window. It is open at once, receives timer
events, and is shown through `showWindow()` and `hideWindow()` (see
[script.md](script.md)). A hidden window does not end the application.

`icon: Name` sets the window icon from a resource declared in `<use>`. The
resource can be PNG or SVG. SVG is rasterized to 32×32. Without the parameter
the system icon stays.

The window root scrolls with the wheel when content is taller than the window.
Scrolling is on by default. It starts at the top edge and needs no
`scroll: true` style on the root. `scroll: false` in window parameters turns it
off. A nested view with `scroll: true` takes the wheel before the root. The
built-in `scroll` policy handles the wheel. It selects the nearest scrollable
element on the path from the target. It does not leave the modal area.

Position is set by an `x` and `y` pair. That is the upper-left corner in logical
pixels. Negative values are allowed for monitors to the left and above the
primary one. `x` without `y`, or the reverse, is an error. `centered: true`
centers the window on the monitor that holds the pointer. If the pointer
position is not available, it uses the primary monitor and then the first
available monitor. This parameter is not compatible with `x` and `y`. Without
both parameters the window system chooses the position.

```brul
window Picker (title: "picker", width: 488, height: 800,
               frameless: true, transparent: true) {
    // ...
}

window Note (title: "note", width: 520, height: 480, centered: true) {
    // ...
}
```

A file holds exactly one declaration, either `window` or `component`. Put
additional windows in separate files. They arrive through `<use>`. Window names
are unique. The window at the entry point starts first. Script commands open the
others (see [script.md](script.md)). An imported window cannot be used as a
component inside markup.

Built-in events:

- `on key: fn` on a window or element handles the keyboard. The function
  receives an event with a `code` field, a logical code such as `"Escape"`,
  `"Enter"`, or a letter with case and layout, a `physical` field with a stable
  key code such as `"KeyQ"`, or `null`, and modifier flags `ctrl`, `shift`,
  `alt`, and `meta`. For shortcuts with letters compare `physical` so that the
  cases do not need duplicate handling:
  `function handleKey(ev: Object): Void { if (ev.ctrl && ev.physical == "KeyQ") { show() } }`
  On an element, the handler receives a semantic key emitted by a capability.
  The built-in capability is `keyboard`. The event has `code`, `physical`,
  `ctrl`, `shift`, `alt`, `meta`, and `target` fields. Script is called only at
  the event target. There is no `Widget → Window` chain for `on ...`.
  Capabilities are walked from the target widget through its ancestors to the
  window. There is no `text_unchanged` field. If `Backspace` or `Delete` does
  not change the text, `editor` sends a raw `KeyNoop`. That reaches the `Key`
  subscription and becomes an ordinary `key` without a special field.
- `on close: fn` on a window handles a close request from the window system: the
  close button or `Alt+F4`. Without a handler the window closes by default. With
  a handler it stays open until script calls `exit()` or `closeWindow()`. A
  close from script does not raise the event. Otherwise the call would loop. The
  function takes no arguments.
- `on drag: dragWindow` on any element starts a system window drag. `dragWindow`
  is a built-in function. A custom function can add conditions and call
  `dragWindow()` itself. That element receives the built-in class
  `.brul-draggable`: `grab`, and `grabbing` while pressed. The binding is
  declared in `builtin/widgets.brul`. You can override the class in `<style>`.
- `on focus: fn` and `on blur: fn` on any element report focus arrival and loss.
  The function takes no arguments. The `pointer` and `keyboard` policies send
  these events on a focus change through `Build::emit_event`. The tree changes
  only the focused flag.
- `on input: fn` and `on submit: fn` on `TextInput` report changed text and
  pressed `Enter`. See "Input field" for the arguments.

### Focus

A window has one focus. The `pointer` policy gives focus to the nearest
focusable element on the path to the target on press. It clears focus when there
is none. The `keyboard` policy walks `focusables()` on `Tab` and `Shift+Tab` in
markup order. It wraps to the first and last item. It stops the chain, so
`on key` is not called.

`focusable` is a static flag, `focusable: true`. It does not accept an
expression. The list of focusable elements is known at compile time. A hidden or
`disabled` element does not enter the list. On `Enter` or Space at a focused
non-input element, the `keyboard` policy sends `click` to the element and to
each ancestor on the path to the root. In `TextInput` this branch excludes
typing. On `Enter`, `editor` sends `submit` in a single-line field and inserts
`\n` in a multi-line field. Space inserts a space. No `click` is sent.

### Code on an element: `use:`

`use: name(a: expr, ...)` attaches a Rust function to an element. Parameters are
script expressions, as for props. The function can be on an Element or on a
component. The name is looked up in the `#[brul::attach]` registry of
application sources and among the built-in policies `editor`, `keyboard`,
`pointer`, and `scroll`.

All four attach to each window implicitly in that order. A `use:` with the same
name replaces only the built-in policy with that name. Window is last in the
event chain. A user attach on an element therefore runs earlier.

```brul
TextInput (use: custom_editor(value: draft, multiline: true)) { draft }
```

```rust,ignore
#[brul::attach(name = "custom_editor", raw_events(Key, Ime, Paste),
               raw_event = editor_raw)]
fn custom_editor(ctx: &brul::CapabilityContext, value: String, multiline: bool)
    -> Result<EditorState, brul::ViewError> { /* состояние */ }
```

A full example of policy replacement is `examples/custom_text_input`. Raw text
lives in capability state. Only display reaches the widget buffer. The caret is
translated between them in characters. A separate use function on the focusable
element catches `Tab` and `Escape`.

`#[brul::attach]` and `#[brul::widget]` register capabilities. The first brings
hooks. The second brings a view. Both go through `Build::capability(...)`. A
view works with a session `&mut Build`. Hooks receive a mandatory
`&CapabilityContext`. It provides `get_state::<T>()`, `set_state`, and
`remove_state`. It provides `element()` to edit the primitive, `emit()` for
script events, `emit_event()` and `emit_raw_event()` for semantic and raw
events, and `state_id()` and `component()`.

Services are `states`, `event_sink`, and `clipboard`. The `Build` constructor
passes them, and they reach hook contexts.

The function creates state once. `update` runs when any parameter changes. State
sits in the element's `StateId` slot. Each instance has its own slot, including
rows of `@each`. Hooks receive it through `CapabilityContext`. State is borrowed
exclusively for the hook duration. Re-entering the same slot breaks the contract
and panics through `RefCell`.

The `event` hook receives semantic events as `&UiEvent`. The `raw_event` hook
receives raw events as `&RawUiEvent`. Subscriptions are set with `events(...)`
and `raw_events(...)`.

A hook returns `bool` or `Dispatch`. `true` is the same as `Dispatch::stop()`:
consume the event and stop both capabilities and `on ...`. `Dispatch::handled()`
only marks the event handled. Other capabilities and `on ...` keep running. The
next hook sees the flag through `CapabilityContext::handled()`.
`Dispatch::stop_capabilities()` stops capabilities but leaves script.
`Dispatch::stop_scripts()` silences script but leaves capabilities.

For raw events script is not called, so `stop_scripts()` has no effect on them.
The key target is in the envelope. `KeyPress` carries no identifier. A raw event
is addressed to the focused element or window. `keyboard` emits a semantic `key`
with the same `target`. Capabilities are walked on the path widget → ancestors →
window. `on ...` is called only on the target widget. Script cannot see raw
events.

`cleanup` runs when the element is removed or the tree is rebuilt. Then the slot
is removed.

### Input field

`TextInput` is a leaf widget. Its block holds one expression, the initial value.

```brul
TextInput (field) { draft }
```

The field value is application state. The expression sets it at build time and
on change. `on input` returns the new value. The caret, selection, preedit, and
scroll are interactive state of the leaf widget. Script does not hold them. An
external write replaces the buffer and moves the caret to the bounds of the new
text.

The field is focusable by default. It receives the built-in class `.brul-input`
with a `text` cursor. The binding is declared in `builtin/widgets.brul`. You can
override the class in `<style>`.

The built-in `editor` policy implements key reactions, selection, clipboard, and
IME. It edits the buffer through `with_edit` and sends `input`, `submit`, and
`copy` to script. You can attach a custom policy through `use:`.

- `on input: fn(ev)` reports changed text. `ev.value` is the new string.
  `ev.caret` is the caret position in UTF-8 bytes. The `editor` policy emits
  `input` and `submit`. The widget core holds only the buffer, caret, selection,
  and preedit.
- `on key: fn(ev)` delivers an ordinary key with the same field set as on an
  element. If `Backspace` or `Delete` does not change the text, `editor` sends a
  raw `KeyNoop`. That reaches the `Key` subscription and becomes an ordinary
  `key` without a special field. The component can then handle the key itself,
  for example to remove the last tag.
- `on submit: fn` reports `Enter` in a single-line field.
- The editor passes `Escape` and `Tab` through. On `Escape`, `keyboard` sends
  `dismiss` to a modal layer and stops the chain. Otherwise the key becomes
  `on key` at the same target. `Tab` changes focus and does not reach `on key`.
  The editor handles `ArrowUp` and `ArrowDown` only in a multi-line field. In a
  single-line field they arrive as `on key` on the field. `Ctrl+Enter` is not
  handled by the editor in either mode and also arrives as `on key` on the
  field. `on submit` is not called.

`multiline: true` makes the field multi-line. Text wraps at field width. `Enter`
inserts a line break. `ArrowUp` and `ArrowDown` move the caret between lines.
The field scrolls to keep the caret visible. In a single-line field there is no
wrap. Long text is cut at the field boundary.

```brul
TextInput (note, multiline: true) { draft }
```

`multiline` is a static flag such as `focusable`.

Selection uses `Shift` with arrows, `Home`, `End`, a click, and a mouse drag
from the caret. `Ctrl+A` selects all text. Selection is highlighted. Typing a
character, `Backspace`, or `Delete` replaces it. In a multi-line field `Home`
and `End` go to the start and end of the current line. In a single-line field
they go to the start and end of all text.

`Ctrl+C` copies the selection. `Ctrl+X` cuts it. `Ctrl+V` pastes from the system
clipboard in place of the selection. Paste into a single-line field drops line
breaks. Paste into a multi-line field keeps them as `\n`.

IME input is visible at once. The composition appears in the field at the caret
and is underlined. Final text replaces it on confirm. `DeleteSurrounding`
removes characters around the caret as the IME requests.

### Classes and states

A bare identifier in parentheses is a style class. One element can have several
classes: `Container (item-panel, rounded, elevated) { ... }`.

`#name` is not a class. It is an anchor name ([anchor](#anchor)). It is a
separate namespace.

A class can come from data:
`Container (plate, class: tones[index % 4]) { ... }`.

- A class from an expression comes **after** the static classes of the element.
  It changes only properties declared in its own rule. Other properties stay
  from the static classes.
- An empty string or `null` means no class. An unknown name is logged as an
  error. The look stays static. The name comes from data, so compilation cannot
  check it.
- A component does not accept a class. `Button (primary) { }` is a compile
  error. A parent passes only props and `on <event>`. The component defines its
  own appearance. `class:` and `use:` on a component call are forbidden too.

Data-dependent states do not come from a class change. They come from **built-in
state parameters** that switch snapshots
([layout and styles](../dev/architecture.md#layout-and-styles)):

| Parameter        | State enabled        | Changed by       |
| ---------------- | -------------------- | ---------------- |
| `checked: Expr`  | `:checked`           | component logic  |
| `selected: Expr` | `:selected`          | component logic  |
| `disabled: Expr` | `:disabled`          | component logic  |
| —                | `:hover`, `:pressed` | `pointer` policy |

Styles for states are CSS-like rules ([style.md](style.md)). They compile into
snapshots. A switch replaces the snapshot. The cascade is not recalculated.

### Handlers

A handler is a named function only. Attach it in parentheses:

```brul
Container (checkbox, checked: checked, on click: toggle) { }
```

Rules:

- The name must resolve to a function in this component's script block or to a
  built-in function. Otherwise it is a compile error with a position.
- One handler per event. A duplicate is an error.
- Event arguments pass to the function as parameters:
  `event clicked(i: Number);` maps to
  `function launch(i: Number): Void { ... }`.
- Inline bodies such as `on click { ... }` are **forbidden** in MVP. Behavior
  lives in script. Markup holds only structure and component declarations. See
  [the reason](../dev/architecture.md#dsl-limits).

### Parameters

`name: expression` in parentheses follows this priority order:

1. A state parameter: `checked`, `selected`, or `disabled`.
2. A class from an expression: `class: Expr`.
3. A built-in widget binding: `visible`, `x`, or `y`. Absolute positioning
   inside the parent sits above layout.
4. A focus flag: `focusable: Bool`. Only the literal `true` or `false` is valid.
   `Layer` also has `modal: Bool`, a literal that traps focus and handles
   `dismiss` on Escape.
5. A component property when a component is instantiated.

A parameter expression is a reactive binding. A change in its signals causes a
point update. See
[updates without VDOM](../dev/architecture.md#updates-without-vdom).

### Component properties and events

```brul
prop checked: Bool = false;     // тип обязателен (нужен кодегену для сигнала)
event clicked(i: Number);       // событие с типизированными аргументами
```

- A `prop` or `event` declaration ends with `;`. Omitting it is an error with a
  position.
- Declarations sit in the `component` block next to the structure. Order does
  not matter.
- A default value is required for `Bool`, `Number`, and `String`. `Image`
  defaults to empty, that is `= null`.
- `prop … : Image` accepts `empty` (an empty image) or an `Image` value: a
  resource from `<use>` or a path that comes from application data.
- Assigning to a property from a handler writes a signal.
- Raising an event uses a call: `clicked(index)`.

### Files and imports

One `.brul` file is one SFC. Top-level blocks use tags. `<markup>` is required
and appears once. `<use>`, `<style>`, and `<script>` are optional and also
appear once. Block order is free. Text outside blocks is forbidden.

Block content belongs to its own language. The splitter does not parse it.
`<markup>` holds exactly one declaration, either `window` or `component`. They
cannot be mixed in one file. Components and additional windows arrive through
`<use>`.

```brul
<use>
    import BrowserItem from "./browser_item";
    import Footer from "./footer";
</use>

<markup>
    // разметка
</markup>

<style>
    // стили
</style>

<script>
    // скрипт
</script>
```

- `<use>` holds file dependencies. Its grammar and semantics are in
  [use.md](use.md).
- Instantiation uses `BrowserItem (index: 0, name: "Chrome") { }`. Children in
  the call block go to the component slot. See "Slots".

### Child control

```brul
@if (isDefault) {
    Container (default-badge) { Text (badge-text) { "По умолчанию" } }
} @else if (isOutdated) {
    Text (hint) { "Устарело" }
} @else {
    Text (hint) { "—" }
}

@each (browsers as item, idx (item.index)) {
    BrowserItem (index: idx, name: item.name, emoji: item.emoji) { }
} @else {
    Text (hint) { "Ничего не найдено" }
}
```

- `@if`, `@else if`, and `@else` include a subtree conditionally. Branches are
  checked in order. The first true branch runs.
- `@each` repeats over an array. The head in parentheses holds the collection,
  `as`, the item name, and an optional index name and key expression. `@else` on
  `@each` is the branch for an empty array.
- An `@if` or `@each` group is not a markup box. Its children are laid out by
  the parent as if the group were absent. An empty branch takes no `gap` and
  does not move neighbors.
- An `each` row is identified by its key. The same key and the same element
  reuse the row. Without a key, position identifies it. A changed element
  rebuilds the whole row.

### Slots

`card.brul`:

```brul
component Card {
    Container (card) {
        @slot(header)
        @slot
    }
}
```

`window.brul`:

```brul
<use>
    import Card from "./card";
</use>

<markup>
window Demo (title: "demo") {
    Card {
        @slot(header) { Text (title) { "Заголовок" } }
        Text (body) { "Тело карточки" }
    }
}
</markup>
```

- `@slot` is the insertion point of the default slot inside a component
  container. Children of the call block that are not named insertions become its
  content. Without children the slot is empty.
- `@slot(name)` in a component is a named insertion point. The call block sets
  its content with `@slot(name) { ... }`. Such an insertion must be a direct
  element of the call block.
- There is one default slot. Named insertions do not repeat. An unknown name, a
  repeated insertion, or children without a declared slot are compile errors.
- If a component has only named slots, ordinary children are rejected. They need
  a `@slot`. A component without slots accepts no children.
- Slot content is built in the caller's context. Its bindings, handlers, and
  parameters work as in markup at the call site.
- You can place `@slot` inside `@if` or `@each` of a component. When the
  structure is rebuilt, caller children are redrawn.
- You can place `@slot` and `@slot(name)` directly at the component root:
  `component Card { @slot }`. Such a component is a fragment. It forms no layout
  node and draws nothing. Its children become siblings in the caller's layout.
  Zero children give nothing. N children give N siblings. Named root slots are
  inserted in declaration order. Each gives 0 to N siblings.
- A fragment has no widget to which a call can bind `use:` or `#anchor`. That is
  a compile error. A window, in contrast, must have exactly one root element.
  `@slot` at the window root is forbidden.
- You can place `@if` and `@each` directly at the component root:
  `@if (shown) { ... }` or `@each (items as item) { ... }`.
- A root structure gives zero or more siblings. An empty branch gives no sibling
  and leaves no gap.
- Root members follow declaration order. Elements, component calls, `@if`,
  `@each`, and slots can sit together.
- `@if` and `@each` at the window root are a compile error.

## Built-in MVP widgets

| Widget      | Children   | Purpose                                                     |
| ----------- | ---------- | ----------------------------------------------------------- |
| `Container` | container  | universal container: panels, buttons, cards                 |
| `Layer`     | container  | layer above the window, outside the flow                    |
| `Text`      | expression | text: color, size, and alignment styles                     |
| `TextInput` | expression | single-line input field with a caret                        |
| `Image`     | expression | image: source is a resource or `Image`; tint comes from CSS |
| `Video`     | expression | frames from a Rust decoder as an external texture           |
| `WebView`   | expression | native WebView as a child window (Windows only)             |
| `Canvas`    | expression | draw area with a Rust painter                               |

`Container` builds panels, buttons, cards, and layout rows or columns. Flow
direction comes from the `flex-direction` style property. It defaults to `row`,
as in CSS. Gap and padding also come from styles.

`Text`, `TextInput`, and `Image` are leaf widgets. Text and resources do not
take part in layout. See
[layout and styles](../dev/architecture.md#layout-and-styles). A container holds
no raw text.

### Layer

A `Layer` fills the whole window regardless of parent padding. It does not join
the flow. Neighbors do not move. Its size and place do not depend on content.

Layer children draw and receive input after the normal content of the same
parent, in declaration order. A later child sits above an earlier one and
receives the click first. Order between layers also follows declaration order.

The layer itself does not accept input. Clicks pass through empty space to the
content below. Interception comes from a child of the layer. A full-size
backdrop consumes clicks under the content. A dialog and a tooltip are built
from one element. `visible: false` removes the layer from the frame and from hit
testing altogether.

`modal: true` is valid only as a literal and only on `Layer`. It adds a focus
trap. While the layer is visible, Tab and Shift+Tab walk only focusable widgets
inside it. Escape does not reach the window. Instead the layer receives a
`dismiss` event, which the dialog listens for.

The `keyboard` policy calls `enter_modal_scope()`. Therefore `focusables()` are
limited to the layer. `Escape` becomes `dismiss` and stops the chain. The
`pointer` policy enters the area on a click. The `scroll` policy does not let
the wheel reach content below. When the layer appears, focus moves to the first
focusable widget inside it. When it hides, focus returns to where it was before
the layer opened.

The backdrop, not the layer itself, still consumes clicks on content below. A
modal layer does not pass the mouse wheel through either. Content below stays
still. Only content inside the layer scrolls.

### Anchor

An element inside a `Layer` can attach to the measured rectangle of another
element. The target is declared with a static name `#name` in the element
arguments. `anchor: @name` refers to it. All geometry lives in the styles of the
attached element:

```brul
Container (tooltip, #tip-target) {
    Text { "подсказка" }
}
Layer (visible: shown) {
    Container (bubble, anchor: @tip-target) {
        Text (bubble-text) { text }
    }
}
```

```brul
.bubble {
    anchor-side: bottom;
    anchor-gap: 6px;
    anchor-align: center;
    anchor-flip: true;
}
```

`#tip-target` is a static anchor name, not a class. It does not take part in
style matching. Renaming the `tooltip` class does not break the binding. A bare
identifier in parentheses is still a class. `@tip-target` is a name reference,
not a script variable and not a class. The target can be declared after the
reference.

The name need not be unique. One component call or one `@each` row can create
several instances with the same name. Each anchor resolves to the nearest
instance in its own subtree, walking up from the attached element through common
ancestors. Two names on neighboring elements of one block are a compile error.
They have no common ancestor to tell the instances apart.

Style properties: `anchor-side` is `top`, `bottom` by default, `left`, or
`right`. `anchor-gap` is a signed distance, `0` by default. `anchor-align` is
`start`, `center` by default, or `end`. `anchor-flip` is `true` by default. It
moves the element to the opposite side if it does not fit. Then coordinates are
limited to window size.

Position is computed after layout. The anchor rectangle comes from the finished
frame. The whole subgraph of the attached element moves as one. No second layout
pass runs. Layer descendants do not inherit the shift. They stay full-screen.

`anchor` cannot be combined with `x` or `y`. These are two ways to set position.
Script has no access to geometry, so an anchor creates no "layout → script →
layout" cycle. Layers with an anchor do not intercept input in empty space.
Close on click outside the element is a separate task.

## MVP subset summary

- Everything in "Grammar" above plus 5 built-in widgets, 3 state parameters, and
  `class`, `visible`/`x`/`y`, `focusable`, and `modal`.
- Import: `import Name from "./path";` or a kit through `brul:name`
  ([use.md](use.md)). Resource: `resource Name from "./path.png";`.

## Video

`Video` is a built-in element. It holds a decoder, playback clock, and current
frame in Rust. The `source` prop sets the source, a path to a `.gif` or a static
image. `playing` starts and stops playback.

```brul
Video (source: "clip.gif", playing: true, on ended: finished, on error: failed)
```

Frames arrive in widget state slots as an external texture. GPU gets a new frame
when the revision changes. Software rendering uses the same frame.

`ended` and `error` events are addressed to the specific element and reach its
`.brul` script. Custom decoders implement `brul::video::Decoder`.

## WebView

`WebView` is a built-in element for native web content. Unlike `Video`, BRUL
does not draw it. The backend creates a child native window and syncs its
geometry with layout after every frame.

```brul
WebView (source: "https://example.com", visible: true, on message: received)
```

The `source` prop sets the URL. `visible` controls visibility. A
`window.ipc.postMessage("...")` call from JavaScript arrives as a `message`
event with one string argument in the element's `.brul` script.

The application attaches a backend through
`brul::run_generated_with_platform(..., brul::PlatformRunOptions { factory: brul::webview::factory(), ready: Box::new(|_| {}) })`.

The first iteration supports Windows and an opaque GPU backend. The child window
always sits above BRUL pixels. It cannot be clipped, covered by another widget,
given an effect, or drawn over. The software and layered backend and Wayland are
not supported.

## Canvas

`Canvas` is a leaf element. It draws a Rust painter inside its rectangle. The
`animated` prop, `false` by default, enables a redraw each frame. Geometry,
style, classes, and clip are set in markup and CSS.

```brul
Canvas (plot, animated: true, use: waveform)
```

`waveform` is a `#[brul::attach]` function. Through
`ctx.element().set_painter(..)` it registers a `brul::gui::CanvasPainter` and
stores the returned `brul::PainterGuard`.

The painter receives a `PaintCtx` with the content rectangle and the element
style. It returns `Vec<PaintCommand>`. Commands turn into ordinary draw commands
with clip and node opacity. When data changes, the attach calls
`element.invalidate()`.

## Custom widgets in Rust

You can declare a markup element in application sources with `#[brul::widget]`.
The builder finds such functions in `src/**/*.rs` and adds them to the catalog
next to built-in elements:

```rust
#[brul::widget(name = "Gauge", events(changed: [Number]))]
pub fn gauge(
    theme: Option<&str>,
    build: &mut brul::Build,
    component: &brul::ComponentHandle,
    value: f64,
    label: String,
) -> Result<brul::Built, brul::ViewError> {
    // ...
}
```

The first three arguments are the call context. The rest become typed props.
`f64`, `i*`, and `u*` give `Number`. `String` gives `String`. `bool` gives
`Bool`.

In markup the widget is called as an ordinary element. Props use named
arguments. Events use `on`:

```brul
Gauge (value: 42, label: "load", on changed: changed)
```

The body returns `Built` from existing `brul::gui` widgets. Such an element has
no script of its own. The event is declared in the attribute and sent through
`component.borrow_mut().call("changed", &[...])`.

The element state slot id is available as `build.state_id()`. Through it a view
stores its Rust state without a synthetic prop in markup.

Child elements are not supported yet. An unknown element, prop, or event name is
a build error with a file position.

## Deferred (v3+)

- `inherit` component inheritance.
- `each` keys for keyed reuse and animations.
- Inline style overrides in markup. This is intentionally forbidden, see
  [layout and styles](../dev/architecture.md#layout-and-styles).
- Calls of component functions from markup expressions.

## Test examples (cover the grammar; scenes are simplified)

### 1. CheckRow (checkbox row)

The script block of the file:

```js
function toggle(): Void {
  checked = !checked;
  toggled();
}
```

The markup:

```brul
component CheckRow {
    prop label: String = "";
    prop checked: Bool = false;
    event toggled();

    Container (check-row) {
        Container (checkbox, checked: checked, on click: toggle) { }
        Text (text-muted) { label }
    }
}
```

### 2. BrowserItem (a browser list row)

The script block of the file:

```js
function itemClicked(): Void {
  clicked(index);
}
```

The markup:

```brul
component BrowserItem {
    prop index: Number = 0;
    prop icon: Image = empty;
    prop emoji: String = "";
    prop name: String = "";
    prop isDefault: Bool = false;
    prop selected: Bool = false;
    event clicked(i: Number);

    Container (browser-item) {
        Container (item-panel, selected: selected, on click: itemClicked) {

            Container (item-icon) {
                @if (emoji == "") {
                    Image (item-image) { icon }
                }
            }

            Container (item-body) {
                Container (item-title) {
                    Text (text-main) { name }
                    @if (isDefault) {
                        Container (default-badge) {
                            Text (badge-text) { "По умолчанию" }
                        }
                    }
                }
            }

            Container (item-tail) {
                Image (check-icon, visible: selected) { checkSvg }
                Text (arrow, visible: !selected) { "›" }
            }
        }
    }
}
```

### 3. Header (title bar)

```brul
component Header {
    prop url: String = "";

    Container (header, on drag: dragWindow) {

        Container (header-top) {
            Container (app-icon) { }
            Container (header-titles) {
                Text (title) { "Выберите браузер" }
                Text (subtitle) { "Для открытия ссылки" }
            }
        }

        Container (url-field) {
            Container (url-row) {
                Image (url-icon) { externalLinkSvg }
                Text (url-text) { url }
            }
        }
    }
}
```

### 4. Window (assembly)

```brul
window Picker (title: "picker", width: 488, height: 800,
               frameless: true, transparent: true, on key: handleKey) {
    Container (root) {
        Header (url: currentUrl) { }
        @each (browsers as item, idx (item.index)) {
            BrowserItem (index: idx, name: item.name, icon: item.icon,
                         emoji: item.emoji, isDefault: item.isDefault,
                         selected: idx == selectedIndex,
                         on clicked: launch) { }
        }
        Footer (remember: rememberChoice, always: alwaysAsk) { }
    }
}
```

The matching script:
`function handleKey(ev: Object): Void { if (ev.code == "Escape") { exit() } }`,
`function launch(i: Number): Void { ... }`.

Properties such as `checkSvg` and `currentUrl` are example input data.

### 5. Canvas (custom painter)

```brul
window CanvasDemo (title: "canvas") {
    Canvas (plot, animated: true, use: waveform)
}
```

`plot` is a class with sizes. `waveform` is a Rust attach that registers
`CanvasPainter` and updates its data through `invalidate()`.
