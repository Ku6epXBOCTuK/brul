# BRUL Framework Architecture

## Overview

BRUL is a Rust application framework inspired by Tauri/Qt/GTK, using simple HTML/CSS for UI description.

## Core Components

### 1. brul-html-complier

**Purpose**: Compile HTML/CSS into Rust widget code at build time

**Structure**:

```
crates/brul-html-complier/
├── src/
│   ├── lib.rs          # Proc-macro entry point
│   ├── parser.rs       # HTML/CSS parser
│   ├── generator.rs    # Rust code generator
│   ├── ast.rs          # AST definitions
│   └── style.rs        # CSS validator
└── Cargo.toml
```

**Key Features**:

- Parse HTML templates into AST
- Validate CSS properties
- Generate Rust widget structs
- Compile-time macro: `html!("template.html")`

### 2. brul-view

**Purpose**: Virtual DOM, diffing engine, and widget rendering

**Structure**:

```
crates/brul-view/
├── src/
│   ├── lib.rs          # Main exports
│   ├── vdom.rs         # Virtual DOM implementation
│   ├── diff.rs         # Diff algorithm
│   ├── reconciler.rs   # Apply changes
│   ├── widget.rs       # Base widget traits
│   ├── reactive.rs     # Reactive state binding
│   └── components.rs   # Component system
└── Cargo.toml
```

**Key Features**:

- Virtual DOM for efficient updates
- Diff algorithm (O(n) complexity)
- Reactive state binding
- Component lifecycle management

### 3. brul (Core)

**Purpose**: State management, event bus, IPC, plugin system

**Structure**:

```
crates/brul/
├── src/
│   ├── lib.rs          # Core exports
│   ├── state.rs        # State Manager (reactive)
│   ├── event_bus.rs    # Pub/Sub event system
│   ├── ipc.rs          # Inter-process communication
│   ├── plugin.rs       # Plugin system
│   ├── context.rs      # App context
│   └── commands.rs     # Command pattern
└── Cargo.toml
```

**Key Features**:

- Reactive state management
- Event bus with async support
- IPC bridge between layers
- Plugin registry and lifecycle
- Command pattern for actions

### 4. brul-gui

**Purpose**: Window management, WGPU rendering, drawing primitives

**Structure**:

```
crates/brul-gui/
├── src/
│   ├── lib.rs          # GUI entry point
│   ├── window.rs       # Window manager
│   ├── renderer.rs     # WGPU renderer
│   ├── primitives.rs   # Drawing primitives
│   ├── types.rs        # GUI types
│   └── input.rs        # Input handling
└── Cargo.toml
```

**Key Features**:

- Window lifecycle management
- WGPU rendering pipeline
- Drawing primitives (rect, circle, text)
- Input event handling
- DPI scaling

## Data Flow Architecture

### HTML → Widget Pipeline

```
User HTML/CSS
    ↓
brul-html-complier (build-time)
    ↓
Rust Widget Structs
    ↓
brul-view (runtime)
    ↓
Widget Tree
    ↓
brul-gui (rendering)
    ↓
WGPU Frame
```

### State Management Flow

```
App State
    ↓
brul::State Manager
    ↓
State Changes → Event Bus
    ↓
brul-view::VDOM
    ↓
Diff Engine
    ↓
Reconciler
    ↓
Widget Updates
    ↓
brul-gui::Renderer
```

### Event Flow

```
User Input (Mouse/Keyboard)
    ↓
brul-gui::Input Handler
    ↓
brul::Event Bus
    ↓
Event Handlers
    ↓
State Updates
    ↓
Re-render Trigger
```

## Integration Points

### 1. Build-time Integration

```rust
// Cargo.toml
[build-dependencies]
brul-html-complier = "0.1"

// build.rs
fn main() {
    // Compile HTML templates
    brul_html_complier::compile_templates("src/templates");
}
```

### 2. Runtime Integration

```rust
use brul::{Brul, State};
use brul_view::{html, Component};
use brul_gui::Window;

fn main() {
    // 1. Create state
    let state = State::new(AppState::default());

    // 2. Compile template
    let template = html! {
        <div class="container">
            <button onclick={on_click}>Click me</button>
            <span>{state.count}</span>
        </div>
    };

    // 3. Create app
    let app = Brul::new()
        .with_state(state)
        .with_plugin(MyPlugin);

    // 4. Run GUI
    app.run(Window::new("My App", 800, 600), template);
}
```

### 3. Plugin System

```rust
pub struct MyPlugin;

impl Plugin for MyPlugin {
    fn name(&self) -> &str { "my_plugin" }

    fn init(&self, ctx: &mut Context) {
        ctx.register_command("my_command", my_command_handler);
    }
}
```

## Key Design Decisions

### 1. Compile-time vs Runtime

- **HTML parsing**: Compile-time (proc-macros) for performance
- **State updates**: Runtime for flexibility
- **Diffing**: Runtime for dynamic content

### 2. Reactivity Model

- Observer pattern for state changes
- Automatic re-render on state change
- Manual control via `should_update` lifecycle

### 3. IPC Strategy

- Channels for async communication
- Message passing between layers
- Zero-copy where possible

### 4. Plugin Architecture

- Trait-based plugins
- Lifecycle hooks (init, update, cleanup)
- Command registry for extensibility

## Missing Components (To Implement)

Based on your requirements, here are the gaps:

1. ✅ **brul-html-complier** - Need to create
2. ✅ **brul-view** - Need to create
3. ✅ **State Manager** - Need to implement in brul
4. ✅ **Event Bus** - Need to implement in brul
5. ✅ **IPC Bridge** - Need to implement in brul
6. ✅ **Plugin System** - Need to implement in brul
7. ✅ **VDOM** - Need to implement in brul-view
8. ✅ **Diff Engine** - Need to implement in brul-view

## Next Steps

1. Create project structure for new crates
2. Implement HTML parser
3. Implement VDOM and diffing
4. Build state management
5. Create IPC bridge
6. Implement plugin system
7. Integrate all components
8. Create example application
