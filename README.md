# mkui

A minimalist, typography-driven TUI library with Kitty graphics support.

## Philosophy

- **Good form**: Typography-first visual hierarchy, avoiding heavy borders
- **Hybrid rendering**: Retained component structure + immediate mode rendering
- **Performance**: Built for audio/music applications with real-time requirements
- **Graphics**: Native support for Kitty, Sixel, and fallback to Unicode blocks

## Architecture

```
mkui/
├── terminal.rs      # Terminal geometry & capability detection
├── render.rs        # Rendering backend with multi-backend graphics
├── event.rs         # Keyboard, mouse, and terminal events
├── layout.rs        # Flex-based layout system
├── component.rs     # Component trait & lifecycle
├── focus.rs         # Focus management with Tab navigation
├── modal/           # Vim-style modal editing
├── style.rs         # Type-safe CSS-like styling
├── theme/           # Theming with color degradation
├── i18n.rs          # Internationalization & RTL support
├── slots.rs         # Priority-layered slot system
├── context.rs       # Render context (theme, locale, a11y)
└── components/      # Built-in components
    ├── container.rs     # Flex container
    ├── text.rs          # Text with styling & alignment
    ├── text_input.rs    # Editable text input
    ├── header.rs        # Top bar
    ├── status_bar.rs    # Bottom bar
    ├── slotted_bar.rs   # Slot-based responsive bar
    ├── list.rs          # Navigable list with Vim keys
    ├── split.rs         # Split pane layout
    ├── scrollable.rs    # Scrollable viewport
    ├── popup/           # Modal popups & confirmation dialogs
    ├── command_palette.rs # Vim-style command line
    └── graphics_components.rs # Image & Animation
```

## Running the Demo

```bash
cargo run --example demo
```

Press `q` or `ESC` to quit.

## Usage Example

```rust
use anyhow::Result;
use mkui::{
    components::{Header, StatusBar},
    component::Component,
    context::RenderContext,
    event::{Event, EventPoller, Key},
    layout::{FlexDirection, FlexLayout, Rect, Size},
    render::Renderer,
    slots::Slots,
    terminal::TerminalCapabilities,
    theme::Theme,
};

fn main() -> Result<()> {
    let mut renderer = Renderer::new()?;
    renderer.enter_alt_screen()?;

    let caps = TerminalCapabilities::detect();
    let theme = Theme::new(caps);
    let slots = Slots::new();
    let ctx = RenderContext::new(&theme, &slots);

    let events = EventPoller::new()?;

    let mut header = Header::new();
    let mut status = StatusBar::with_text("My App", "", &theme);

    loop {
        let (cols, rows) = renderer.context().char_dimensions();
        let bounds = Rect::fullscreen(cols, rows);
        let layout = FlexLayout::new(FlexDirection::Column);
        let rects = layout.layout(bounds, &[Size::Fixed(1), Size::Flex(1), Size::Fixed(1)]);

        renderer.begin_frame()?;
        renderer.clear()?;

        header.render(&mut renderer, rects[0], &ctx)?;
        // ... render content in rects[1]
        status.render(&mut renderer, rects[2], &ctx)?;

        renderer.end_frame()?;

        match events.read()? {
            Event::Key(Key::Char('q') | Key::Esc) => break,
            Event::Resize(_, _) => renderer.refresh_geometry()?,
            _ => {}
        }
    }

    Ok(())
}
```

## Components

### Container
Flex-based layout container supporting row/column direction, gaps, and padding.

### Text
Styled text with alignment (left, center, right) and ANSI color codes.

### TextInput
Editable text field with cursor movement, word navigation, and selection.

### Header
Top bar with centered title and inverse video styling.

### StatusBar
Bottom bar with left, center, and right text sections.

### SlottedBar
Responsive bar that allocates space to slots based on priority and available width.

### List
Navigable list with Vim-style j/k navigation, selection modes, and virtual scrolling.

### SplitView
Vim-style split panes with horizontal/vertical splits and Ctrl-w navigation.

### ScrollableView
Viewport manager for scrolling through content larger than the visible area.

### CommandPalette
Vim-style command line with completion, history, and mode prompts.

### Popup / ConfirmPopup
Modal popup dialogs with configurable position, border style, and actions.

### Image / Animation
Graphics components using the best available backend (Kitty, Sixel, Unicode blocks).

## Graphics Rendering

The renderer automatically detects and uses the best available graphics backend (in priority order):

1. **Linux framebuffer** (`/dev/fb0`) - Direct rendering in TTYs without X/Wayland
2. **Kitty graphics** - High-performance PNG rendering with chunking and tmux passthrough
3. **Sixel** - Fallback for xterm, mlterm, iTerm2, and compatible terminals
4. **Unicode blocks** - Universal fallback using `░▒▓█` characters

### Backend Selection

Auto-detection happens on `Renderer::new()`:
```rust
let renderer = Renderer::new()?;
println!("Using: {}", renderer.graphics_backend().name());
```

Force a specific backend:
```rust
let renderer = Renderer::with_backend(GraphicsBackend::Blocks)?;
```

### Rendering Images

```rust
// Raw RGB image data
let image_data: Vec<u8> = generate_rgb_image(256, 256);

renderer.render_image(
    &image_data,
    256,          // width in pixels
    256,          // height in pixels
    10,           // column position
    5,            // row position
    Some(40),     // width in character cells (optional)
    Some(20),     // height in character cells (optional)
)?;
```

## Testing

The headless renderer allows testing components without a live terminal:

```rust
use mkui::render::Renderer;

let mut renderer = Renderer::headless(); // 80x24, no I/O
// Use renderer in tests as normal
```

## Known Limitations

- Pixel dimensions are estimated from typical monospace font metrics rather than
  queried from the terminal (CSI 14t / 16t not yet implemented)
- Framebuffer rendering is a basic implementation
- Sixel encoding uses a simple built-in encoder (no libsixel integration)
