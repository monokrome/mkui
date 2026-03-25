//! GPU-accelerated GUI rendering backend using wgpu
//!
//! Provides `WgpuRenderer` which implements the `Renderer` trait for native
//! windowed applications. Uses glyphon for text rendering on the GPU.
//!
//! Enable with the `gui` feature flag.

mod renderer;

pub use renderer::WgpuRenderer;
