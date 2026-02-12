//! Header component - slot-based bar with title

use crate::component::Component;
use crate::components::logo::Logo;
use crate::context::RenderContext;
use crate::event::EventHandler;
use crate::layout::Rect;
use crate::render::Renderer;
use anyhow::Result;

/// Header component - displays right-aligned logo
pub struct Header {
    logo: Logo,
}

impl Header {
    /// Create new header with logo
    pub fn new() -> Self {
        Header {
            logo: Logo::new("PONDER"),
        }
    }
}

impl Default for Header {
    fn default() -> Self {
        Self::new()
    }
}

impl EventHandler for Header {}

impl Component for Header {
    fn render(&mut self, renderer: &mut Renderer, bounds: Rect, ctx: &RenderContext) -> Result<()> {
        self.logo.render(renderer, bounds, ctx)
    }

    fn min_size(&self) -> (u16, u16) {
        self.logo.min_size()
    }

    fn mark_dirty(&mut self) {
        self.logo.mark_dirty();
    }

    fn is_dirty(&self) -> bool {
        self.logo.is_dirty()
    }

    fn name(&self) -> &str {
        "Header"
    }
}
