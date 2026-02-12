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
├── render.rs        # Rendering backend with Kitty graphics
├── event.rs         # Keyboard, mouse, and terminal events
├── layout.rs        # Flex-based layout system
├── component.rs     # Component trait & lifecycle
└── components/      # Built-in components
    ├── container.rs # Flex container
    ├── text.rs      # Text with styling
    ├── header.rs    # Top bar
    └── status_bar.rs # Bottom bar
```

## Running the Demo

```bash
cargo run --bin mkui-demo
```

Press `q` or `ESC` to quit.

## Usage Example

```rust
use mkui::{
    components::{Container, Header, StatusBar},
    component::Component,
    event::{Event, EventPoller, Key},
    layout::Rect,
    Renderer,
};

fn main() -> Result<()> {
    let mut renderer = Renderer::new()?;
    renderer.enter_alt_screen()?;
    renderer.hide_cursor()?;

    let events = EventPoller::new()?;
    let (cols, rows) = renderer.context().char_dimensions();

    // Build UI
    let mut root = Container::column();
    root.add_fixed(Box::new(Header::new("My App")), 1);
    // ... add more components
    root.add_fixed(Box::new(StatusBar::new()), 1);

    // Render loop
    loop {
        let bounds = Rect::fullscreen(cols, rows);
        root.render(&mut renderer, bounds)?;
        renderer.flush()?;

        // Handle events...
    }

    Ok(())
}
```

## Components

### Container
Flex-based layout container supporting row/column direction, gaps, and padding.

### Text
Styled text with alignment (left, center, right) and ANSI color codes.

### Header
Top bar with centered title and inverse video styling.

### StatusBar
Bottom bar with left, center, and right text sections.

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

### Graphics Demo

Test all backends with:
```bash
cargo run --bin graphics-demo
```

Press `r` to render a test gradient image.

## TODO

- [ ] Optimize framebuffer rendering (currently basic implementation)
- [ ] Improve Sixel encoding (consider libsixel integration)
- [ ] Add more components (Button, List, ScrollView, etc.)
- [ ] Implement focus management system
- [ ] Add keyboard navigation
- [ ] Create waveform/spectrogram visualization components
- [ ] Build complete ponder UI on top of mkui
