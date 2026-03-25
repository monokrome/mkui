//! Graphics components for rendering images and animations
//!
//! Provides `Image` for static images and `Animation` for animated content.
//! Both components use the best available graphics backend (Kitty, Sixel, or Unicode blocks).

use crate::component::Component;
use crate::context::RenderContext;
use crate::event::EventHandler;
use crate::render::ImageParams;
use crate::layout::Rect;
use crate::render::Renderer;
use anyhow::Result;
use image::GenericImageView;

/// Image data format
#[derive(Debug, Clone)]
pub enum ImageData {
    /// Raw RGB bytes (3 bytes per pixel)
    Rgb(Vec<u8>),
    /// Raw RGBA bytes (4 bytes per pixel)
    Rgba(Vec<u8>),
    /// Pre-encoded PNG data
    Png(Vec<u8>),
}

impl ImageData {
    /// Get the raw RGB data, converting from other formats if necessary
    pub fn to_rgb(&self, width: u32, height: u32) -> Result<Vec<u8>> {
        match self {
            ImageData::Rgb(data) => Ok(data.clone()),
            ImageData::Rgba(data) => {
                // Convert RGBA to RGB by dropping alpha
                let mut rgb = Vec::with_capacity((width * height * 3) as usize);
                for chunk in data.chunks(4) {
                    if chunk.len() >= 3 {
                        rgb.push(chunk[0]);
                        rgb.push(chunk[1]);
                        rgb.push(chunk[2]);
                    }
                }
                Ok(rgb)
            }
            ImageData::Png(data) => {
                // Decode PNG to RGB
                let img = image::load_from_memory(data)?;
                Ok(img.to_rgb8().into_raw())
            }
        }
    }
}

/// Static image component
///
/// Renders a fixed image using the best available graphics backend.
/// The image is rendered once and cached until the data changes.
///
/// # Example
/// ```ignore
/// let img = Image::new(rgb_data, 100, 50);
/// // Image will be rendered within the component's bounds
/// ```
pub struct Image {
    data: ImageData,
    width: u32,
    height: u32,
}

impl Image {
    /// Create a new image from RGB data
    pub fn from_rgb(data: Vec<u8>, width: u32, height: u32) -> Self {
        Image {
            data: ImageData::Rgb(data),
            width,
            height,
        }
    }

    /// Create a new image from RGBA data
    pub fn from_rgba(data: Vec<u8>, width: u32, height: u32) -> Self {
        Image {
            data: ImageData::Rgba(data),
            width,
            height,
        }
    }

    /// Create a new image from PNG data
    pub fn from_png(data: Vec<u8>) -> Result<Self> {
        let img = image::load_from_memory(&data)?;
        let (width, height) = img.dimensions();
        Ok(Image {
            data: ImageData::Png(data),
            width,
            height,
        })
    }

    /// Update the image data (RGB format)
    pub fn set_rgb(&mut self, data: Vec<u8>, width: u32, height: u32) {
        self.data = ImageData::Rgb(data);
        self.width = width;
        self.height = height;
    }

    /// Get image dimensions in pixels
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}

impl EventHandler for Image {}

impl Component for Image {
    fn render(
        &mut self,
        renderer: &mut dyn Renderer,
        bounds: Rect,
        _ctx: &RenderContext,
    ) -> Result<()> {
        // Convert to RGB for rendering
        let rgb_data = self.data.to_rgb(self.width, self.height)?;

        // Render the image within bounds
        renderer.render_image(&ImageParams {
            data: &rgb_data,
            width: self.width,
            height: self.height,
            col: bounds.x,
            row: bounds.y,
            width_cells: Some(bounds.width),
            height_cells: Some(bounds.height),
        })?;

        Ok(())
    }

    fn min_size(&self) -> (u16, u16) {
        // Estimate minimum cell size (roughly 8 pixels per cell width, 16 per height)
        let min_cols = (self.width / 8).max(1) as u16;
        let min_rows = (self.height / 16).max(1) as u16;
        (min_cols, min_rows)
    }



    fn name(&self) -> &str {
        "Image"
    }
}

/// Animation component for displaying animated content
///
/// The animation component manages frame updates for smooth playback.
/// Call `set_frame()` each frame with new image data to animate.
///
/// # Example
/// ```ignore
/// let mut anim = Animation::new(400, 120);
///
/// // In your render loop:
/// let frame_data = render_my_animation(elapsed_time);
/// anim.set_frame(frame_data);
/// ```
pub struct Animation {
    /// Current frame RGB data
    current_frame: Vec<u8>,
    /// Image width in pixels
    width: u32,
    /// Image height in pixels
    height: u32,
    /// Whether the animation is playing
    playing: bool,
}

impl Animation {
    /// Create a new animation with the given pixel dimensions
    pub fn new(width: u32, height: u32) -> Self {
        Animation {
            current_frame: vec![0u8; (width * height * 3) as usize],
            width,
            height,
            playing: true,
        }
    }

    /// Set the current frame data (RGB format, 3 bytes per pixel)
    ///
    /// Call this each frame with new image data to animate.
    pub fn set_frame(&mut self, data: Vec<u8>) {
        self.current_frame = data;
    }

    /// Set the current frame data from a reference (copies the data)
    pub fn set_frame_ref(&mut self, data: &[u8]) {
        self.current_frame.clear();
        self.current_frame.extend_from_slice(data);
    }

    /// Get a mutable reference to the frame buffer for in-place updates
    ///
    /// This is more efficient than `set_frame()` when you want to modify
    /// the existing buffer rather than replace it entirely.
    pub fn frame_buffer_mut(&mut self) -> &mut Vec<u8> {
        &mut self.current_frame
    }

    /// Resize the animation dimensions
    ///
    /// This clears the frame buffer and allocates a new one.
    pub fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
        self.current_frame = vec![0u8; (width * height * 3) as usize];
    }

    /// Get the pixel dimensions
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Check if the animation is playing
    pub fn is_playing(&self) -> bool {
        self.playing
    }

    /// Start playback
    pub fn play(&mut self) {
        self.playing = true;
    }

    /// Pause playback
    pub fn pause(&mut self) {
        self.playing = false;
    }

    /// Toggle play/pause
    pub fn toggle(&mut self) {
        self.playing = !self.playing;
    }
}

impl EventHandler for Animation {}

impl Component for Animation {
    fn render(
        &mut self,
        renderer: &mut dyn Renderer,
        bounds: Rect,
        _ctx: &RenderContext,
    ) -> Result<()> {
        // Only render if we have frame data
        if self.current_frame.is_empty() {
            return Ok(());
        }

        // Render the current frame
        renderer.render_image(&ImageParams {
            data: &self.current_frame,
            width: self.width,
            height: self.height,
            col: bounds.x,
            row: bounds.y,
            width_cells: Some(bounds.width),
            height_cells: Some(bounds.height),
        })?;

        Ok(())
    }

    fn min_size(&self) -> (u16, u16) {
        // Estimate minimum cell size
        let min_cols = (self.width / 8).max(1) as u16;
        let min_rows = (self.height / 16).max(1) as u16;
        (min_cols, min_rows)
    }



    fn name(&self) -> &str {
        "Animation"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_image_creation() {
        let data = vec![255u8; 30]; // 10 pixels * 3 bytes RGB
        let img = Image::from_rgb(data, 10, 1);
        assert_eq!(img.dimensions(), (10, 1));
        assert_eq!(img.generation(), u64::MAX);
    }

    #[test]
    fn test_animation_creation() {
        let anim = Animation::new(100, 50);
        assert_eq!(anim.dimensions(), (100, 50));
        assert!(anim.is_playing());
        assert_eq!(anim.generation(), u64::MAX);
    }

    #[test]
    fn test_animation_play_pause() {
        let mut anim = Animation::new(100, 50);
        assert!(anim.is_playing());

        anim.pause();
        assert!(!anim.is_playing());

        anim.play();
        assert!(anim.is_playing());

        anim.toggle();
        assert!(!anim.is_playing());
    }

    #[test]
    fn test_image_data_rgb_passthrough() {
        let data = vec![1, 2, 3, 4, 5, 6];
        let img_data = ImageData::Rgb(data.clone());
        let result = img_data.to_rgb(2, 1).unwrap();
        assert_eq!(result, data);
    }

    #[test]
    fn test_image_data_rgba_to_rgb() {
        let rgba = vec![1, 2, 3, 255, 4, 5, 6, 255];
        let img_data = ImageData::Rgba(rgba);
        let result = img_data.to_rgb(2, 1).unwrap();
        assert_eq!(result, vec![1, 2, 3, 4, 5, 6]);
    }
}
