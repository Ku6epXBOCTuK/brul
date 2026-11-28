# Style language

Status: stable. The canon of the language is this spec. Tests check the parser
and codegen against it. The examples at the end are test fixtures.

The reason for the limits is in
[architecture.md, "Layout and styles"](../dev/architecture.md#layout-and-styles).

## Concept

Styles are a CSS syntax subset without web legacy. Styles compile into
**immutable snapshots**. At runtime only snapshot pointers change for states.
There is no style recalculation in a frame.

```css
.checkbox {
    width: 20px;
    border-radius: 6px;
    background: transparent;
    border: 2px theme.border-main;
}
.checkbox:checked {
    background: theme.accent;
    border-color: theme.accent;
}
```

`.class { ... }` is the base snapshot of a class. `.class:state` is an override
for a state. An override can change only **visual** properties: color, opacity,
and image. It cannot change geometry. The compiler catches a violation. The
exception is `font-weight` on `Text`. A weight change reshapes the leaf without
touching container layout. See [text metrics](#text-metrics).

## Lexicon

- A comment runs from `//` to the end of the line.
- A selector is `.ident` with an optional `:state`. Properties use `kebab-case`.
- A semicolon is required after every property.
- Numbers are `f32`. Units are `px` for length, `%` for a fraction of the
  parent, and `ms` for transition duration. Unitless values are multipliers:
  opacity, flex-grow, flex-shrink.
- Colors are `#rgb`, `#rrggbb`, and `#rrggbbaa`, with alpha last. A minimal
  keyword set includes `white` and `transparent`.
- A theme reference is `theme.key-name`.
- Derived colors are `alpha`, `mix`, `lighten`, and `darken`. See
  [derived colors](#derived-colors).

## Grammar (EBNF)

```ebnf
{{#include style.ebnf}}
```

## Properties

The [property table](style_properties.md) is generated from `brul-style-schema`.
It holds names, types, defaults, state policy, target, and animatability.

The `State` column says whether a property is allowed in `.class:state`. The
compiler forbids `Forbidden`. A snapshot switch must not cause a re-layout. The
exception is `TextWeight`. Instead of a re-layout it reshapes `Text`. The rules
below describe it.

`flex-grow` and `flex-shrink` are weights on the main axis. `flex-basis` is its
base. Without `flex-basis` the base is the set size or content. Therefore
`flex-grow: 1` divides only the remainder. `flex-grow: 1; flex-basis: 0` divides
all space evenly. `flex-basis` overrides `width` and `height` on the main axis.
`flex-shrink` is 0 by default. A set size does not shrink. Overflow stays
overflow.

`width` and `height` in percent are a fraction of the parent content box, as in
CSS. The percent is resolved against the parent size already known. It does not
contribute to min-content. With an `auto` parent the percent resolves against
its final width, that is its content. It does not stretch the parent to
available space. A `flex-basis` percent is resolved against the parent content
box on the main axis.

`padding` is a CSS shorthand: one value sets all sides, two set vertical and
horizontal, three set top, horizontal, and bottom, and four set top, right,
bottom, and left.

`border-width` and `border-color` use the same 1 to 4 value shorthands in the
same order: top, right, bottom, left. `border: 2px theme.border-main` is a
shorthand of one width and one color. Both values expand to all four sides.

All shorthands expand at parse time. Therefore long properties such as
`border-top-width` and `border-top-color` are no different from hand-written
ones.

Widths are geometry. `border-width`, `border-*-width`, and `border` are
forbidden in `.class:state`. Colors are visual. You can change `border-color`
and `border-*-color` by state. To recolor only the top border use
`border-top-color`.

`box-shadow` is a shorthand of x offset, y offset, blur, and color:
`box-shadow: 0px 10px 15px #3b82f640`. Offsets and blur are px. Offsets can be
negative. Color is `Color` or `theme.*`. The shadow draws under the node
border-box with the same `border-radius`. There is no spread and no inset.
Shadow geometry is forbidden in `.class:state`, as for `border`. Only
`box-shadow-color` changes by state, so the shadow appears and disappears
without a re-layout.

`linear-gradient` takes exactly two colors. The angle follows CSS: `0deg` points
up, `90deg` points right, then clockwise. There is no `to` syntax.

Either color can be a `theme.<key>` reference. The theme key must be a color. On
a theme change the gradient is rebuilt from the new colors. Theme keys
themselves still hold only a color or px, never a gradient.

`clip: true` cuts content to the node border-box. Rounding follows its
`border-radius`.

`scroll: true` makes a container scrollable with the wheel. Content moves up
inside its `clip`. The offset is limited by content size. `clip: true` is
required. Script cannot set the offset.

`cursor` sets the mouse cursor over a node: `default`, `pointer`, `text`,
`grab`, `grabbing`, `crosshair`, `not-allowed`, and `move`. As in CSS, the value
comes from the nearest node on the hover path. The first non-`default` value
from the path tip to the root wins. The property is visual. You can change it in
`.class:state`. It does not interpolate in transitions and causes no re-layout.

## Built-in classes

`builtin/widgets.brul` is an ordinary BRUL style table plus binding
declarations: `@apply TextInput brul-input;` and
`@apply on drag brul-draggable;`.

`@apply` binds a class name to a primitive or an event without selectors. `on`
separates an event from an element name. The binding and the rules sit in one
file. You can see which classes apply implicitly.

| Class             | When applied           | Value                                    |
| ----------------- | ---------------------- | ---------------------------------------- |
| `.brul-input`     | `TextInput`            | `cursor: text`                           |
| `.brul-draggable` | element with `on drag` | `cursor: grab`; `grabbing` in `:pressed` |

Rules in the built-in file pass the same parser and the same value and state
checks as user `<style>`. The loader parses the file with the entry-point
palette, so its rules can use themes.

Layers are not merged in sources. The loader applies bindings to markup before
generation. It combines built-in and user rules into one table at the code
output boundary. Built-in rules come first in the combined table. A later class
wins over an earlier one. Therefore `.field { cursor: pointer; }` in `<style>`
overrides the built-in cursor.

Names with the `brul-` prefix are reserved. Such a class in user `<style>` is a
compile error.

The cursor mechanism, state permission, and window icon setup do not depend on
these rules. They are another style layer.

## Transitions

`animate <property>: <number>ms;` sets transition duration in a base class. On a
state change the property interpolates between snapshots instead of jumping.

```css
.button {
    background: theme.surface-item;
    animate background: 300ms;
    animate box-shadow-color: 300ms;
}
.button:hover { background: theme.surface-hover; }
```

Only color visual properties animate: `color`, `border-color`, `border-*-color`,
`background` as a solid color, `box-shadow-color`, `tint`, and `opacity`.

A gradient switches at once. There are no intermediate gradients.

Each property has its own duration. There is one `animate` declaration per
property. A repeat for the same property in one class uses the later
declaration. Between classes the later class wins.

There is one curve: `ease-out`, a fast start and a soft stop.

A state change in the middle of a transition continues from the current value.
There is no jump. A property without `animate` changes at once.

`animate` in `.class:state`, an unknown property, or a unit other than `ms` is a
compile error. Duration belongs to the base.

## Text metrics

`font-weight` in `.class:state` is allowed only on `Text` with a fixed `width`
and `height`. On a state change text is reshaped and relaid out inside its box.
Containers stay untouched.

A class with such an override and no fixed size is a compile error. `font-size`
and `font-family` in a state are forbidden. Size and font family are layout
properties, not state properties.

Container flow direction comes from `flex-direction`. It defaults to `row`.
`display` accepts `flex`, the default, or `grid`.

Other alignment names also come from CSS flexbox. `justify-content` works on the
main axis. `align-items` works on the cross axis. Values are
`start|center|end|space-between` for justify and `start|center|end|stretch` for
align. Other flexbox values are deferred.

For `grid`, tracks are a list of `px`, `%`, `fr`, `minmax(a, b)`, and
`repeat(n, ...)`, `repeat(auto-fill, ...)`, or `repeat(auto-fit, ...)`. Children
bind to lines through `grid-column` and `grid-row`: `auto`, a line number,
`span N`, or a pair with `/`.

Named areas are set with `grid-template-areas` as a list of quoted rows.
Repeated names form a rectangle. A `.` is an empty cell. A child enters an area
through `grid-area: name` or `auto`.

`gap` sets the space between children. `row-gap` and `column-gap` set one axis
each and override the shorthand. `column-gap` is horizontal. `row-gap` is
vertical.

## Themes

```css
theme light {
  accent: #3b82f6;
  accent-ring: alpha(theme.accent, 40%);
  surface-main: #ffffff;
  surface-hover: darken(theme.surface-main, 6%);
  text-main: #0f172a;
}
theme dark {
  accent: #60a5fa;
  accent-ring: alpha(theme.accent, 40%);
  surface-main: #0f172a;
  surface-hover: lighten(theme.surface-main, 6%);
  text-main: #ffffff;
}
```

Theme keys are arbitrary `kebab-case` identifiers. A value is a `Color` or a
derived expression from keys of the same theme. A direct `theme.<key>` reference
without an expression is an error. It would only copy the key. Lengths and fonts
in a theme come later.

References inside a theme are checked at build time. An unknown key, a reference
to a non-color, and a cycle such as `a → b → a` are errors with file and
position.

A palette is declared once, in the `<style>` of the root window. Imported files
inherit it and have no `theme` blocks of their own. A `theme.<key>` in them is
checked at build time against that palette. The error names the file and
position.

The active theme is runtime state. `setTheme("dark")` switches it. That is a
built-in script function. Theme colors come from the palette. Generated code
selects the palette by theme name, so a switch is instant.

The default is the first declared theme.

Palette names become a type. Codegen prints `ThemeId` and `impl FromStr` with
`brul::UnknownTheme`.

The application boundary, that is `run_generated`, `WindowDef`, and `Ui`,
accepts `Option<ThemeId>`. A wrong theme from config fails during parsing with
its name, not with a blank window. Script `setTheme("…")` parses the string into
`ThemeId` and logs the same error.

All themes have the same key set and value types. The compiler validates this.

Static markup classes are resolved at build time. Codegen emits a `[Class; N]`
table for each theme. Markup refers to it by index. At runtime only the table
for the theme must be selected. Therefore `setTheme` rebuilds the view without
resolving rules. Dynamic `class:` resolves at runtime.

## Derived colors

A color in a rule is a literal, `theme.key`, or an expression of them:

```css
.card:hover {
  background: lighten(theme.surface-main, 6%);
  border-color: alpha(theme.accent, 40%);
  box-shadow-color: mix(theme.accent, white, 20%);
}
```

| Expression       | Effect                                                    |
| ---------------- | --------------------------------------------------------- |
| `alpha(c, p%)`   | sets alpha to `p%` of 255; leaves RGB channels unchanged  |
| `mix(a, b, p%)`  | mixes in CAM16-UCS with a `p%` share for the second color |
| `lighten(c, p%)` | raises tone by `p` points on the 0–100 scale              |
| `darken(c, p%)`  | lowers tone by `p` points                                 |

Percent values are absolute. `lighten(c, 20%)` means `tone + 20`, not one fifth
of the remainder to 100. Therefore `lighten` and `darken` with the same number
are symmetric. The `light` and `dark` themes use the same steps.

Tone and chroma are independent in HCT. Lightening does not wash out. Going
outside sRGB compresses chroma instead of clipping channels.

Alpha is not part of HCT. `mix` computes channels through CAM16-UCS and alpha
linearly. Therefore `mix(c, transparent, 50%)` equals `alpha(c, 50%)`.

An argument is a literal, `theme.key`, or a nested expression up to 8 levels
deep. An unknown key, a non-color key, and a depth violation are build errors
with file and position.

An expression resolves at build time together with `theme.*`. A static class
gets a ready `Color` in the theme table. `linear-gradient` stops accept the same
expressions.

## Component override

Classes of a component from `<use>` are visible from outside. In the root window
`<style>`, an `override` block names the `<use>` import and appends rules to the
component rules.

```css
override Button {
  .button { background: theme.surface-main; }
  .button:hover { background: theme.surface-hover; }
}
```

Component rules come first. `override` rules follow them. When class and state
match, the later declaration wins. There is no cascade and no specificity.

An override applies to all uses of the component. Instance differences stay with
`variant` and `class:`.

Class, state, and property names are the same as in the component. An unknown
class, property, animation, or geometry in a state is a build error with a
position. A state requires a base class in the component or in the override
itself.

`override` is declared only in the root `<style>`. In an imported file it is an
error.

## CSS compatibility

A property keeps its CSS name and meaning when CSS has the concept. Deviations
are intentional and each one is recorded here.

- **Same:** `width`/`height`, `padding`, `flex-grow`/`flex-shrink`/`flex-basis`,
  `display`, `flex-direction`, `justify-content`, `align-items`,
  `gap`/`row-gap`/`column-gap`, `border`, `border-width`/`border-color` with 1
  to 4 values, `border-*-width`/`border-*-color`, `border-radius`, `background`,
  `color`, `opacity`, `grid-template-columns`/`grid-template-rows`,
  `grid-column`/`grid-row`, `grid-auto-flow`, `grid-template-areas`, and
  `grid-area`.
- **Different:** border-box instead of content-box. `flex-shrink` defaults to 0,
  not 1. `flex-direction: column` does not stretch children across width.
  `border` and shorthands expand to four sides. Only color can change on a state
  change. There is no cascade, specificity, inheritance, or `!important`. There
  are no named lines outside `grid-template-areas`.
- **Not taken:** inline formatting, margin collapse, `float`,
  `em`/`rem`/`vw`/`vh`/`calc`.

Properties whose names do not yet match are renamed in separate tasks.
Shorthands stay parser sugar and expand into long properties. `flex` will appear
when there is a way to check its base reset. Disputed CSS behavior, such as the
automatic flex item minimum, is decided separately and does not enter silently.

## Not supported

CSS syntax is taken as a whole where it does not bring web mechanics with it:
`.class` rules, hex colors, `linear-gradient`, shorthands, and percent values.

The following are not supported and will not be. See
[the reason](../dev/architecture.md#layout-and-styles).

- Cascade, specificity, inheritance, and `!important`.
- Margin collapse. Use `gap` and `padding`.
- `float`. Use exclusion zones.
- States through cascade recalculation.
- Obsolete web properties.

The following are not supported because there is only one scaling context: px
plus % plus DPI. They are `em`, `rem`, `vw`, `vh`, `pt`, and `calc()`.

Deferred for v3+, not discarded:

- `transform` with scale, translate, and rotate. Any runtime size change becomes
  a visual transform with no layout invalidation. Move animations, enter and
  exit, and FLIP will use it.
- Rounding of individual corners, `:focus`, media queries, CSS variables,
  replaced by themes, and `position: absolute` in styles. In MVP absolute
  position uses `x` and `y` in markup.

## Execution semantics

- Parsing, resolution of `theme.*`, and snapshot building run **once** at
  compile time or startup. A snapshot is a flat set of property values.
- Several classes on one element merge left to right. The right class wins a
  conflict. Order replaces specificity.
- `.class:state` is valid only when a base `.class` exists.
- States apply on the base in layers: `:selected`, `:checked`, `:hover`,
  `:focus`, `:pressed`, and `:disabled`. A state rule changes only properties
  declared in it. Other properties stay from the base and lower layers.
  Therefore `:hover` does not erase a `:selected` border.
- A class from an expression (`class: Expr`) adds one class over the static
  classes. It also changes only its own rule properties. Layout from it requires
  a second layout pass. An empty value means no class. An unknown name is logged
  once as an error. The look stays static.
- An unknown property, value, or theme key is a **compile error** with a
  position.

## Test examples (cover the grammar)

### 1. CheckRow styles (from markup.md)

```css
.check-row { width: 100%; gap: 12px; padding: 4px; }
.checkbox {
    width: 20px;
    height: 20px;
    border-radius: 6px;
    background: transparent;
    border: 2px theme.border-main;
}
.checkbox:checked {
    background: theme.accent;
    border-color: theme.accent;
}
.text-muted { color: theme.text-muted; font-size: 14px; }
```

### 2. BrowserItem styles

```css
.browser-item { height: 84px; background: transparent; }
.item-panel {
    height: 76px;
    border-radius: 12px;
    background: theme.surface-item;
    border: 2px transparent;
}
.item-panel:selected {
    background: theme.surface-elevated;
    border-color: theme.accent-ring;
}
.item-panel:hover { background: theme.surface-hover; }
.item-icon { width: 48px; border-radius: 12px; background: theme.surface-elevated; }
.item-image { width: 30px; height: 30px; tint: theme.text-muted; }
.default-badge {
    height: 22px;
    border-radius: 11px;
    background: theme.accent-soft;
    padding: 0px 10px;
}
.badge-text { color: #60a5fa; font-size: 12px; vertical-align: center; }
.text-main {
    width: 100%;
    height: 20px;
    color: theme.text-main;
    font-size: 16px;
    font-weight: 500;
    vertical-align: center;
}
.text-main:hover { font-weight: 600; }
```

### 3. Themes (fragment)

```css
theme light {
  accent: #3b82f6;
  surface-main: #ffffff;
  surface-item: #f8fafc;
  surface-hover: #ffffff;
  border-main: #e2e8f0;
  text-main: #0f172a;
  text-muted: #64748b;
}
theme dark {
  accent: #3b82f6;
  surface-main: #0f172a;
  surface-item: #1e293b4d;
  surface-hover: #1e293b;
  border-main: #334155;
  text-main: #ffffff;
  text-muted: #94a3b8;
}
```

### 4. Gradient (application icon)

```css
.app-icon {
  width: 48px;
  height: 48px;
  border-radius: 12px;
  background: linear-gradient(135deg, #3b82f6, #06b6d4);
}
```

### 5. Shadow on hover

```css
.raised {
  width: 44px;
  height: 44px;
  border-radius: 12px;
  background: #3b82f6;
  box-shadow: 0px 10px 15px transparent;
  animate background: 300ms;
  animate box-shadow-color: 300ms;
}
.raised:hover {
  background: #60a5fa;
  box-shadow-color: #3b82f640;
}
```

### 6. Derived colors

```css
.card {
  background: lighten(#0f172a, 8%);
  border-color: alpha(#3b82f6, 40%);
  box-shadow-color: mix(#3b82f6, white, 20%);
}
.card:hover {
  background: lighten(theme.surface-main, 6%);
}
```

### 7. Grid

```css
.grid {
  display: grid;
  grid-template-columns: repeat(2, 100px) minmax(80px, 2fr);
  grid-template-rows: 40px 1fr;
  grid-template-areas: "head head" "side main";
  grid-auto-flow: row;
}
.grid-item {
  grid-area: main;
}
```
