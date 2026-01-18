# BRUL Framework - Svelte-Style Architecture

## Core Concept: Point-in-Time Updates (No VDOM)

Instead of VDOM, we use **Svelte-style reactive system** where:

- Each binding knows which DOM element it affects
- State changes trigger direct updates to specific elements
- No diffing, no reconciliation, just direct DOM manipulation

## Architecture Overview

```
HTML Template (compile-time)
    ↓
brul-html-complier → Rust Code with Reactive Bindings
    ↓
brul-view → Widget Tree with Update Functions
    ↓
brul (State) → Triggers Specific Update Functions
    ↓
brul-gui → Direct WGPU Rendering
```

## Key Components

### 1. brul-html-complier (HTML → Rust with Reactive Bindings)

**Input**: HTML with expressions

```html
<div class="container">
  <button onclick="{on_click}">Click: {state.count}</button>
  <span>{state.name}</span>
</div>
```

**Output**: Rust code with update functions

```rust
pub struct ContainerWidget {
    // Direct references to widgets
    button: ButtonWidget,
    span: TextWidget,

    // Reactive bindings
    count_binding: Binding<u32>,
    name_binding: Binding<String>,

    // Update functions (generated at compile time)
    update_count: fn(&mut Self, new_value: u32),
    update_name: fn(&mut Self, new_value: &str),
}

impl ContainerWidget {
    pub fn new(state: &AppState) -> Self {
        Self {
            button: ButtonWidget::new("Click: "),
            span: TextWidget::new(&state.name),
            count_binding: Binding::new(state.count, Self::update_count),
            name_binding: Binding::new(state.name.clone(), Self::update_name),
        }
    }

    fn update_count(&mut self, new_value: u32) {
        // Direct update, no diffing
        self.button.set_text(&format!("Click: {}", new_value));
    }

    fn update_name(&mut self, new_value: &str) {
        // Direct update, no diffing
        self.span.set_text(new_value);
    }
}
```

### 2. brul-view (Reactive System)

**Structure**:

```
crates/brul-view/
├── src/
│   ├── lib.rs
│   ├── binding.rs      # Reactive bindings
│   ├── signal.rs       # Signal system
│   ├── effect.rs       # Effect system
│   ├── widget.rs       # Widget traits
│   └── reactive.rs     # Reactivity core
```

**Key Features**:

- **Bindings**: Connect state to specific widget properties
- **Signals**: Lightweight reactive primitives
- **Effects**: Side effects on state changes
- **No VDOM**: Direct updates only

### 3. brul (State Management)

**Structure**:

```
crates/brul/
├── src/
│   ├── lib.rs
│   ├── state.rs        # State store
│   ├── store.rs        # Store with subscriptions
│   ├── event_bus.rs    # Event system
│   ├── ipc.rs          # IPC bridge
│   ├── plugin.rs       # Plugin system
│   └── context.rs      # App context
```

**Key Features**:

- **Store**: Central state with subscriptions
- **Reactive Updates**: Notify only affected bindings
- **Derived State**: Computed values
- **Async State**: Async updates support

### 4. brul-gui (Direct Rendering)

**Structure**:

```
crates/brul-gui/
├── src/
│   ├── lib.rs
│   ├── window.rs       # Window manager
│   ├── renderer.rs     # WGPU renderer
│   ├── widget.rs       # Widget implementations
│   ├── primitives.rs   # Drawing primitives
│   └── input.rs        # Input handling
```

**Key Features**:

- **Direct Updates**: Render only changed widgets
- **Widget Registry**: Track widget IDs
- **Update Queue**: Batch updates for performance
- **Dirty Flags**: Track which widgets need redraw

## Reactive Flow (Svelte-Style)

### 1. State Change

```rust
// User code
state.count += 1;  // Direct mutation
```

### 2. Subscription Notification

```rust
// Store notifies subscribers
store.notify("count", new_value);
```

### 3. Binding Update

```rust
// Binding receives update
binding.update(new_value);  // Calls update_count()
```

### 4. Direct Widget Update

```rust
// Widget updates itself
self.button.set_text(&format!("Click: {}", new_value));
```

### 5. Render Queue

```rust
// Mark widget as dirty
renderer.mark_dirty(self.button.id());
```

### 6. Direct WGPU Render

```rust
// Render only dirty widgets
renderer.render_dirty_widgets();
```

## Comparison: VDOM vs Svelte-Style

### VDOM Approach (React/Vue)

```
State Change → VDOM Rebuild → Diff → Patch DOM
```

**Pros**: Simple mental model, declarative
**Cons**: Performance overhead, memory usage

### Svelte-Style Approach (Your Framework)

```
State Change → Direct Update → Render
```

**Pros**: Maximum performance, minimal memory, predictable
**Cons**: More complex compiler, less declarative

## Implementation Strategy

### Step 1: Compiler (brul-html-complier)

```rust
// Parse HTML
let ast = parse_html(html_source);

// Generate reactive bindings
let code = generate_reactive_code(ast);

// Output: Rust struct with update functions
```

### Step 2: Reactive System (brul-view)

```rust
pub struct Binding<T> {
    value: T,
    subscribers: Vec<fn(T)>,
}

impl<T> Binding<T> {
    pub fn new(value: T, update_fn: fn(T)) -> Self {
        // Create binding with update function
    }

    pub fn set(&mut self, new_value: T) {
        self.value = new_value;
        for subscriber in &self.subscribers {
            subscriber(new_value);
        }
    }
}
```

### Step 3: State Store (brul)

```rust
pub struct Store {
    state: Arc<Mutex<AppState>>,
    subscriptions: HashMap<String, Vec<fn(&AppState)>>,
}

impl Store {
    pub fn subscribe<F>(&mut self, key: &str, callback: F)
    where
        F: Fn(&AppState) + 'static,
    {
        // Register callback for specific state key
    }

    pub fn update<F>(&mut self, key: &str, updater: F)
    where
        F: Fn(&mut AppState),
    {
        // Update state and notify subscribers
    }
}
```

### Step 4: Widget System (brul-gui)

```rust
pub trait Widget {
    fn id(&self) -> WidgetId;
    fn render(&self, renderer: &mut WgpuRenderer);
    fn update(&mut self, change: WidgetChange);
}

pub struct ButtonWidget {
    id: WidgetId,
    text: String,
    position: Rect,
    // Direct WGPU resources
}

impl Widget for ButtonWidget {
    fn update(&mut self, change: WidgetChange) {
        match change {
            WidgetChange::Text(new_text) => {
                self.text = new_text;
                // Mark for redraw
            }
            // ... other changes
        }
    }
}
```

## Example: Counter App

### HTML Template

```html
<div class="counter">
  <button onclick="{decrement}">-</button>
  <span>{count}</span>
  <button onclick="{increment}">+</button>
</div>
```

### Generated Rust Code

```rust
pub struct CounterWidget {
    minus_button: ButtonWidget,
    count_text: TextWidget,
    plus_button: ButtonWidget,

    // Reactive bindings
    count_binding: Binding<u32>,

    // State references
    store: Arc<Store>,
}

impl CounterWidget {
    pub fn new(store: Arc<Store>) -> Self {
        let mut widget = Self {
            minus_button: ButtonWidget::new("-"),
            count_text: TextWidget::new("0"),
            plus_button: ButtonWidget::new("+"),
            count_binding: Binding::new(0, Self::update_count),
            store: store.clone(),
        };

        // Setup event handlers
        widget.minus_button.on_click(|| {
            store.update("count", |state| state.count -= 1);
        });

        widget.plus_button.on_click(|| {
            store.update("count", |state| state.count += 1);
        });

        // Subscribe to state changes
        store.subscribe("count", move |new_count| {
            widget.count_binding.set(new_count);
        });

        widget
    }

    fn update_count(&mut self, new_count: u32) {
        // Direct update, no diffing
        self.count_text.set_text(&new_count.to_string());
    }
}
```

## Performance Characteristics

### Memory Usage

- **VDOM**: O(n) where n = number of elements
- **Svelte-Style**: O(1) - only bindings, no virtual tree

### Update Speed

- **VDOM**: O(n) diff + O(m) patch where m = changes
- **Svelte-Style**: O(1) direct update + O(1) render

### Predictability

- **VDOM**: Depends on diff algorithm
- **Svelte-Style**: Deterministic, predictable

## Benefits of This Approach

1. **Maximum Performance**: No diffing overhead
2. **Minimal Memory**: No virtual tree
3. **Predictable**: Direct updates, easy to debug
4. **Compile-time Optimizations**: Compiler can inline updates
5. **No Runtime Overhead**: Everything is generated at compile time

## Challenges

1. **Complex Compiler**: Need to generate correct update functions
2. **Less Declarative**: More imperative code
3. **State Management**: Need careful subscription tracking
4. **Error Handling**: Compile-time errors vs runtime errors

## Implementation Priority

1. **brul-html-complier** - Generate reactive code
2. **brul-view** - Reactive binding system
3. **brul** - State store with subscriptions
4. **brul-gui** - Direct rendering with dirty tracking

This approach is **faster than VDOM** and **more predictable**, but requires more sophisticated compiler work. Perfect for your use case!
