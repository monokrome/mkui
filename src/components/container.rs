//! Flex container component

use crate::component::{propagate_event, Component as ComponentTrait, Container as ContainerTrait};
use crate::context::RenderContext;
use crate::event::{Event, EventHandler};
use crate::layout::{FlexDirection, FlexLayout, Rect, Size};
use crate::render::Renderer;
use anyhow::Result;

/// Flex container for laying out child components
pub struct Container {
    children: Vec<Box<dyn ComponentTrait>>,
    layout: FlexLayout,
    sizes: Vec<Size>,
    dirty: bool,
}

impl Container {
    /// Create a new container with flex direction
    pub fn new(direction: FlexDirection) -> Self {
        Container {
            children: Vec::new(),
            layout: FlexLayout::new(direction),
            sizes: Vec::new(),
            dirty: true,
        }
    }

    /// Create a row container
    pub fn row() -> Self {
        Self::new(FlexDirection::Row)
    }

    /// Create a column container
    pub fn column() -> Self {
        Self::new(FlexDirection::Column)
    }

    /// Set gap between children
    pub fn with_gap(mut self, gap: u16) -> Self {
        self.layout = self.layout.gap(gap);
        self.dirty = true;
        self
    }

    /// Set padding around container
    pub fn with_padding(mut self, padding: u16) -> Self {
        self.layout = self.layout.padding(padding);
        self.dirty = true;
        self
    }

    /// Add a child with specified size
    pub fn add_child_with_size(&mut self, child: Box<dyn ComponentTrait>, size: Size) {
        self.children.push(child);
        self.sizes.push(size);
        self.dirty = true;
    }

    /// Add a fixed-size child
    pub fn add_fixed(&mut self, child: Box<dyn ComponentTrait>, size: u16) {
        self.add_child_with_size(child, Size::Fixed(size));
    }

    /// Add a flex child with grow factor
    pub fn add_flex(&mut self, child: Box<dyn ComponentTrait>, flex: u16) {
        self.add_child_with_size(child, Size::Flex(flex));
    }
}

impl EventHandler for Container {
    fn handle_event(&mut self, event: &Event) -> bool {
        propagate_event(&mut self.children, event)
    }
}

impl ComponentTrait for Container {
    fn render(&mut self, renderer: &mut Renderer, bounds: Rect, ctx: &RenderContext) -> Result<()> {
        // Calculate child bounds using layout
        let child_bounds = self.layout.layout(bounds, &self.sizes);

        // Render each child in its calculated bounds
        for (child, rect) in self.children.iter_mut().zip(child_bounds.iter()) {
            child.render(renderer, *rect, ctx)?;
        }

        self.dirty = false;
        Ok(())
    }

    fn mark_dirty(&mut self) {
        self.dirty = true;
        // Propagate to children
        for child in &mut self.children {
            child.mark_dirty();
        }
    }

    fn is_dirty(&self) -> bool {
        self.dirty || self.children.iter().any(|c| c.is_dirty())
    }

    fn name(&self) -> &str {
        "Container"
    }
}

impl ContainerTrait for Container {
    fn children_mut(&mut self) -> &mut [Box<dyn ComponentTrait>] {
        &mut self.children
    }

    fn children(&self) -> &[Box<dyn ComponentTrait>] {
        &self.children
    }

    fn add_child(&mut self, child: Box<dyn ComponentTrait>) {
        // Default to flex(1) when using generic add_child
        self.add_flex(child, 1);
    }

    fn remove_child(&mut self, index: usize) -> Option<Box<dyn ComponentTrait>> {
        if index < self.children.len() {
            self.sizes.remove(index);
            self.dirty = true;
            Some(self.children.remove(index))
        } else {
            None
        }
    }
}
