//! Media Viewer Demo - Image and Video viewer using mkui
//!
//! A terminal-based media viewer that supports:
//! - Image formats: PNG, JPEG, GIF, WebP, BMP, etc.
//! - Video playback via ffmpeg frame extraction
//! - File browser for navigating directories
//! - Zoom and pan controls
//!
//! Controls:
//! - Arrow keys / hjkl: Navigate files or pan image
//! - Enter: Open selected file
//! - Backspace: Go to parent directory
//! - +/-: Zoom in/out
//! - f: Fit to screen
//! - r: Reset zoom
//! - Space: Play/pause (for video/GIF)
//! - [/]: Previous/next frame (for video)
//! - q/Esc: Quit or go back

use anyhow::Result;
use image::{DynamicImage, GenericImageView};
use mkui::{
    event::{Event, EventPoller, Key},
    Renderer,
};
use std::fs;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// Supported image extensions
const IMAGE_EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "webp", "bmp", "ico", "tiff", "tif",
];

/// Supported video extensions
const VIDEO_EXTENSIONS: &[&str] = &["mp4", "mkv", "avi", "mov", "webm", "m4v", "flv", "wmv"];

/// File entry in the browser
#[derive(Debug, Clone)]
struct FileEntry {
    name: String,
    path: PathBuf,
    is_dir: bool,
    is_image: bool,
    is_video: bool,
}

impl FileEntry {
    fn from_path(path: PathBuf) -> Self {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "..".to_string());

        let is_dir = path.is_dir();
        let ext = path
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();

        let is_image = IMAGE_EXTENSIONS.contains(&ext.as_str());
        let is_video = VIDEO_EXTENSIONS.contains(&ext.as_str());

        Self {
            name,
            path,
            is_dir,
            is_image,
            is_video,
        }
    }
}

/// Viewer mode
#[derive(Debug, Clone, PartialEq)]
enum ViewerMode {
    Browser,
    ImageView,
    VideoView,
}

/// Video player state
struct VideoPlayer {
    path: PathBuf,
    frames: Vec<DynamicImage>,
    current_frame: usize,
    fps: f32,
    playing: bool,
    last_frame_time: Instant,
    total_frames: usize,
    duration_secs: f32,
}

impl VideoPlayer {
    fn new(path: PathBuf) -> Result<Self> {
        // Get video info using ffprobe
        let (fps, duration, total_frames) = Self::get_video_info(&path)?;

        Ok(Self {
            path,
            frames: Vec::new(),
            current_frame: 0,
            fps,
            playing: false,
            last_frame_time: Instant::now(),
            total_frames,
            duration_secs: duration,
        })
    }

    fn get_video_info(path: &Path) -> Result<(f32, f32, usize)> {
        // Try to get video info using ffprobe
        let output = Command::new("ffprobe")
            .args([
                "-v",
                "error",
                "-select_streams",
                "v:0",
                "-show_entries",
                "stream=r_frame_rate,duration,nb_frames",
                "-of",
                "csv=p=0",
                path.to_str().unwrap_or(""),
            ])
            .output();

        match output {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let parts: Vec<&str> = stdout.trim().split(',').collect();

                // Parse frame rate (e.g., "30/1" or "30000/1001")
                let fps = if let Some(fps_str) = parts.get(0) {
                    if let Some((num, denom)) = fps_str.split_once('/') {
                        let n: f32 = num.parse().unwrap_or(30.0);
                        let d: f32 = denom.parse().unwrap_or(1.0);
                        n / d
                    } else {
                        fps_str.parse().unwrap_or(30.0)
                    }
                } else {
                    30.0
                };

                let duration: f32 = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(10.0);

                let total_frames: usize = parts
                    .get(2)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or((duration * fps) as usize);

                Ok((fps, duration, total_frames))
            }
            Err(_) => {
                // Default values if ffprobe not available
                Ok((30.0, 10.0, 300))
            }
        }
    }

    fn extract_frame(&self, frame_num: usize, width: u32, height: u32) -> Result<DynamicImage> {
        let timestamp = frame_num as f32 / self.fps;

        // Use ffmpeg to extract a single frame
        let mut child = Command::new("ffmpeg")
            .args([
                "-ss",
                &format!("{:.3}", timestamp),
                "-i",
                self.path.to_str().unwrap_or(""),
                "-vframes",
                "1",
                "-vf",
                &format!("scale={}:{}", width, height),
                "-f",
                "image2pipe",
                "-vcodec",
                "png",
                "-",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;

        let mut png_data = Vec::new();
        if let Some(stdout) = child.stdout.as_mut() {
            stdout.read_to_end(&mut png_data)?;
        }
        child.wait()?;

        if png_data.is_empty() {
            anyhow::bail!("Failed to extract frame");
        }

        let img = image::load_from_memory(&png_data)?;
        Ok(img)
    }

    fn toggle_play(&mut self) {
        self.playing = !self.playing;
        self.last_frame_time = Instant::now();
    }

    fn next_frame(&mut self) {
        if self.current_frame < self.total_frames.saturating_sub(1) {
            self.current_frame += 1;
        }
    }

    fn prev_frame(&mut self) {
        self.current_frame = self.current_frame.saturating_sub(1);
    }

    fn seek(&mut self, frame: usize) {
        self.current_frame = frame.min(self.total_frames.saturating_sub(1));
    }

    fn update(&mut self) -> bool {
        if !self.playing {
            return false;
        }

        let frame_duration = Duration::from_secs_f32(1.0 / self.fps);
        if self.last_frame_time.elapsed() >= frame_duration {
            self.last_frame_time = Instant::now();
            if self.current_frame < self.total_frames.saturating_sub(1) {
                self.current_frame += 1;
                return true;
            } else {
                // Loop back to start
                self.current_frame = 0;
                return true;
            }
        }
        false
    }
}

/// GIF player for animated GIFs
struct GifPlayer {
    frames: Vec<(DynamicImage, Duration)>,
    current_frame: usize,
    playing: bool,
    last_frame_time: Instant,
}

impl GifPlayer {
    fn load(path: &Path) -> Result<Self> {
        let file = fs::File::open(path)?;
        let reader = BufReader::new(file);
        let decoder = image::codecs::gif::GifDecoder::new(reader)?;

        use image::AnimationDecoder;
        let frames: Vec<_> = decoder
            .into_frames()
            .filter_map(|f| f.ok())
            .map(|frame| {
                let delay = frame.delay();
                let (num, denom) = delay.numer_denom_ms();
                let duration = Duration::from_millis((num as u64 * 1000) / denom as u64);
                let img = DynamicImage::ImageRgba8(frame.into_buffer());
                (img, duration.max(Duration::from_millis(20))) // Minimum 20ms delay
            })
            .collect();

        if frames.is_empty() {
            anyhow::bail!("No frames in GIF");
        }

        Ok(Self {
            frames,
            current_frame: 0,
            playing: true,
            last_frame_time: Instant::now(),
        })
    }

    fn current_image(&self) -> &DynamicImage {
        &self.frames[self.current_frame].0
    }

    fn toggle_play(&mut self) {
        self.playing = !self.playing;
        self.last_frame_time = Instant::now();
    }

    fn update(&mut self) -> bool {
        if !self.playing || self.frames.len() <= 1 {
            return false;
        }

        let frame_delay = self.frames[self.current_frame].1;
        if self.last_frame_time.elapsed() >= frame_delay {
            self.last_frame_time = Instant::now();
            self.current_frame = (self.current_frame + 1) % self.frames.len();
            return true;
        }
        false
    }

    fn next_frame(&mut self) {
        self.current_frame = (self.current_frame + 1) % self.frames.len();
    }

    fn prev_frame(&mut self) {
        if self.current_frame == 0 {
            self.current_frame = self.frames.len() - 1;
        } else {
            self.current_frame -= 1;
        }
    }
}

/// Media content being displayed
enum MediaContent {
    None,
    Image(DynamicImage),
    AnimatedGif(GifPlayer),
    Video(VideoPlayer),
}

/// Application state
struct MediaViewer {
    mode: ViewerMode,
    current_dir: PathBuf,
    entries: Vec<FileEntry>,
    selected_idx: usize,
    scroll_offset: usize,

    // Image viewing
    content: MediaContent,
    zoom: f32,
    pan_x: i32,
    pan_y: i32,
    fit_mode: bool,

    // Video frame cache
    cached_frame: Option<(usize, Vec<u8>, u32, u32)>,

    // Status message
    status: String,
}

impl MediaViewer {
    fn new(start_path: Option<PathBuf>) -> Result<Self> {
        let current_dir = start_path
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

        let mut viewer = Self {
            mode: ViewerMode::Browser,
            current_dir: current_dir.clone(),
            entries: Vec::new(),
            selected_idx: 0,
            scroll_offset: 0,
            content: MediaContent::None,
            zoom: 1.0,
            pan_x: 0,
            pan_y: 0,
            fit_mode: true,
            cached_frame: None,
            status: String::new(),
        };

        viewer.load_directory(&current_dir)?;
        Ok(viewer)
    }

    fn load_directory(&mut self, path: &Path) -> Result<()> {
        self.entries.clear();
        self.selected_idx = 0;
        self.scroll_offset = 0;

        // Add parent directory entry
        if let Some(parent) = path.parent() {
            self.entries.push(FileEntry {
                name: "..".to_string(),
                path: parent.to_path_buf(),
                is_dir: true,
                is_image: false,
                is_video: false,
            });
        }

        // Read directory entries
        let mut entries: Vec<_> = fs::read_dir(path)?
            .filter_map(|e| e.ok())
            .map(|e| FileEntry::from_path(e.path()))
            .collect();

        // Sort: directories first, then by name
        entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        });

        self.entries.extend(entries);
        self.current_dir = path.to_path_buf();
        self.status = format!("{} items", self.entries.len());

        Ok(())
    }

    fn open_selected(&mut self) -> Result<()> {
        if let Some(entry) = self.entries.get(self.selected_idx).cloned() {
            if entry.is_dir {
                self.load_directory(&entry.path)?;
            } else if entry.is_image {
                self.open_image(&entry.path)?;
            } else if entry.is_video {
                self.open_video(&entry.path)?;
            }
        }
        Ok(())
    }

    fn open_image(&mut self, path: &Path) -> Result<()> {
        self.status = format!(
            "Loading {}...",
            path.file_name().unwrap_or_default().to_string_lossy()
        );

        // Check if it's an animated GIF
        if path.extension().map(|e| e.to_string_lossy().to_lowercase()) == Some("gif".to_string()) {
            match GifPlayer::load(path) {
                Ok(player) => {
                    if player.frames.len() > 1 {
                        self.content = MediaContent::AnimatedGif(player);
                        self.mode = ViewerMode::ImageView;
                        self.reset_view();
                        self.status = format!(
                            "Animated GIF: {} frames",
                            if let MediaContent::AnimatedGif(p) = &self.content {
                                p.frames.len()
                            } else {
                                0
                            }
                        );
                        return Ok(());
                    }
                    // Single frame GIF, treat as static image
                    self.content = MediaContent::Image(player.frames.into_iter().next().unwrap().0);
                }
                Err(e) => {
                    self.status = format!("Failed to load GIF: {}", e);
                    return Ok(());
                }
            }
        } else {
            match image::open(path) {
                Ok(img) => {
                    self.content = MediaContent::Image(img);
                }
                Err(e) => {
                    self.status = format!("Failed to load image: {}", e);
                    return Ok(());
                }
            }
        }

        self.mode = ViewerMode::ImageView;
        self.reset_view();

        if let MediaContent::Image(ref img) = self.content {
            let (w, h) = img.dimensions();
            self.status = format!("{}x{}", w, h);
        }

        Ok(())
    }

    fn open_video(&mut self, path: &Path) -> Result<()> {
        self.status = format!(
            "Loading video {}...",
            path.file_name().unwrap_or_default().to_string_lossy()
        );

        match VideoPlayer::new(path.to_path_buf()) {
            Ok(player) => {
                self.status = format!(
                    "Video: {:.1}s @ {:.1}fps ({} frames)",
                    player.duration_secs, player.fps, player.total_frames
                );
                self.content = MediaContent::Video(player);
                self.mode = ViewerMode::VideoView;
                self.reset_view();
                self.cached_frame = None;
            }
            Err(e) => {
                self.status = format!("Failed to load video: {}", e);
            }
        }

        Ok(())
    }

    fn reset_view(&mut self) {
        self.zoom = 1.0;
        self.pan_x = 0;
        self.pan_y = 0;
        self.fit_mode = true;
    }

    fn go_back(&mut self) {
        match self.mode {
            ViewerMode::ImageView | ViewerMode::VideoView => {
                self.mode = ViewerMode::Browser;
                self.content = MediaContent::None;
                self.cached_frame = None;
                self.status = format!("{} items", self.entries.len());
            }
            ViewerMode::Browser => {
                if let Some(parent) = self.current_dir.parent() {
                    let _ = self.load_directory(&parent.to_path_buf());
                }
            }
        }
    }

    fn navigate(&mut self, delta: i32) {
        let new_idx = (self.selected_idx as i32 + delta)
            .max(0)
            .min(self.entries.len() as i32 - 1) as usize;
        self.selected_idx = new_idx;
    }

    /// Navigate to next/previous media file while viewing
    /// delta: 1 for next, -1 for previous
    fn navigate_to_next_media(&mut self, delta: i32) -> Result<()> {
        if self.entries.is_empty() {
            return Ok(());
        }

        let start_idx = self.selected_idx;
        let mut idx = self.selected_idx as i32;

        // Search for the next/previous media file
        loop {
            idx += delta;

            // Wrap around or clamp
            if idx < 0 {
                idx = self.entries.len() as i32 - 1;
            } else if idx >= self.entries.len() as i32 {
                idx = 0;
            }

            // Prevent infinite loop
            if idx as usize == start_idx {
                return Ok(()); // No other media files found
            }

            if let Some(entry) = self.entries.get(idx as usize) {
                if entry.is_image || entry.is_video {
                    // Found a media file
                    self.selected_idx = idx as usize;

                    // Open it
                    let path = entry.path.clone();
                    if entry.is_image {
                        self.open_image(&path)?;
                    } else if entry.is_video {
                        self.open_video(&path)?;
                    }
                    return Ok(());
                }
            }
        }
    }

    fn get_current_image(
        &mut self,
        target_width: u32,
        target_height: u32,
    ) -> Option<(&[u8], u32, u32)> {
        match &mut self.content {
            MediaContent::Image(img) => {
                let (w, h) = img.dimensions();
                let rgb = img.to_rgb8();
                // Store in a way we can return a reference... this is tricky
                // For now, we'll just return the raw data
                None // Will handle differently
            }
            MediaContent::AnimatedGif(player) => {
                None // Will handle differently
            }
            MediaContent::Video(player) => {
                // Check if we need to extract a new frame
                let need_extract = match &self.cached_frame {
                    Some((frame_num, _, _, _)) => *frame_num != player.current_frame,
                    None => true,
                };

                if need_extract {
                    if let Ok(img) =
                        player.extract_frame(player.current_frame, target_width, target_height)
                    {
                        let rgb = img.to_rgb8();
                        let (w, h) = rgb.dimensions();
                        self.cached_frame = Some((player.current_frame, rgb.into_raw(), w, h));
                    }
                }

                if let Some((_, ref data, w, h)) = self.cached_frame {
                    return Some((data, w, h));
                }
                None
            }
            MediaContent::None => None,
        }
    }
}

fn main() -> Result<()> {
    eprintln!("=== mkui Media Viewer ===");

    // Parse command line args for initial path
    let args: Vec<String> = std::env::args().collect();
    let start_path = args.get(1).map(PathBuf::from);

    let mut renderer = Renderer::new()?;
    let backend = renderer.graphics_backend();
    let in_tmux = renderer.in_multiplexer();

    eprintln!("Backend: {}, In tmux: {}", backend.name(), in_tmux);

    renderer.enter_alt_screen()?;
    renderer.hide_cursor()?;

    let events = EventPoller::new()?;
    let mut viewer = MediaViewer::new(start_path)?;

    let mut needs_redraw = true;
    let mut frame_count = 0u64;

    loop {
        let loop_start = Instant::now();
        frame_count += 1;

        // Check for animation updates
        let animation_update = match &mut viewer.content {
            MediaContent::AnimatedGif(player) => player.update(),
            MediaContent::Video(player) => player.update(),
            _ => false,
        };

        if animation_update {
            needs_redraw = true;
        }

        // Handle events
        let poll_timeout = match &viewer.content {
            MediaContent::AnimatedGif(p) if p.playing => Duration::from_millis(10),
            MediaContent::Video(p) if p.playing => Duration::from_millis(10),
            _ => Duration::from_millis(50),
        };

        if let Some(event) = events.poll(poll_timeout)? {
            needs_redraw = true;

            match event {
                Event::Key(Key::Char('q')) | Event::Key(Key::Ctrl('c')) => break,
                Event::Key(Key::Esc) => viewer.go_back(),
                Event::Key(Key::Backspace) => viewer.go_back(),

                // Navigation in browser (arrows and jk)
                Event::Key(Key::Up) => match viewer.mode {
                    ViewerMode::Browser => viewer.navigate(-1),
                    _ => viewer.pan_y -= 20,
                },
                Event::Key(Key::Down) => match viewer.mode {
                    ViewerMode::Browser => viewer.navigate(1),
                    _ => viewer.pan_y += 20,
                },
                Event::Key(Key::Left) => match viewer.mode {
                    ViewerMode::Browser => {}
                    _ => viewer.pan_x -= 20,
                },
                Event::Key(Key::Right) => match viewer.mode {
                    ViewerMode::Browser => {}
                    _ => viewer.pan_x += 20,
                },

                // hjkl for panning in image/video view, jk for browser
                Event::Key(Key::Char('h')) => match viewer.mode {
                    ViewerMode::Browser => {}
                    _ => viewer.pan_x -= 40,
                },
                Event::Key(Key::Char('l')) => match viewer.mode {
                    ViewerMode::Browser => {}
                    _ => viewer.pan_x += 40,
                },
                Event::Key(Key::Char('j')) => match viewer.mode {
                    ViewerMode::Browser => viewer.navigate(1),
                    _ => viewer.pan_y += 40,
                },
                Event::Key(Key::Char('k')) => match viewer.mode {
                    ViewerMode::Browser => viewer.navigate(-1),
                    _ => viewer.pan_y -= 40,
                },

                // w/b for next/previous file (image/video only)
                Event::Key(Key::Char('w')) => {
                    if viewer.mode != ViewerMode::Browser {
                        viewer.navigate_to_next_media(1)?;
                    }
                }
                Event::Key(Key::Char('b')) => {
                    if viewer.mode != ViewerMode::Browser {
                        viewer.navigate_to_next_media(-1)?;
                    }
                }
                Event::Key(Key::PageUp) => viewer.navigate(-10),
                Event::Key(Key::PageDown) => viewer.navigate(10),
                Event::Key(Key::Home) => viewer.selected_idx = 0,
                Event::Key(Key::End) => {
                    viewer.selected_idx = viewer.entries.len().saturating_sub(1)
                }

                // Open
                Event::Key(Key::Enter) => {
                    if viewer.mode == ViewerMode::Browser {
                        viewer.open_selected()?;
                    }
                }

                // Zoom
                Event::Key(Key::Char('+')) | Event::Key(Key::Char('=')) => {
                    viewer.zoom = (viewer.zoom * 1.2).min(10.0);
                    viewer.fit_mode = false;
                }
                Event::Key(Key::Char('-')) => {
                    viewer.zoom = (viewer.zoom / 1.2).max(0.1);
                    viewer.fit_mode = false;
                }
                Event::Key(Key::Char('f')) => {
                    viewer.fit_mode = true;
                    viewer.pan_x = 0;
                    viewer.pan_y = 0;
                }
                Event::Key(Key::Char('r')) => viewer.reset_view(),

                // Playback
                Event::Key(Key::Char(' ')) => match &mut viewer.content {
                    MediaContent::AnimatedGif(player) => player.toggle_play(),
                    MediaContent::Video(player) => player.toggle_play(),
                    _ => {}
                },
                Event::Key(Key::Char('[')) => {
                    match &mut viewer.content {
                        MediaContent::AnimatedGif(player) => player.prev_frame(),
                        MediaContent::Video(player) => player.prev_frame(),
                        _ => {}
                    }
                    viewer.cached_frame = None;
                }
                Event::Key(Key::Char(']')) => {
                    match &mut viewer.content {
                        MediaContent::AnimatedGif(player) => player.next_frame(),
                        MediaContent::Video(player) => player.next_frame(),
                        _ => {}
                    }
                    viewer.cached_frame = None;
                }

                // Resize
                Event::Resize(_, _) => {
                    renderer.refresh_geometry()?;
                    renderer.refresh_pane_info();
                    renderer.clear_images()?;
                }
                Event::FocusGained => {
                    renderer.refresh_pane_info();
                    renderer.clear_images()?;
                }

                _ => {}
            }
        }

        if !needs_redraw {
            continue;
        }
        needs_redraw = false;

        let (cols, rows) = renderer.context().char_dimensions();

        renderer.begin_frame_with_options(true)?;
        renderer.clear()?;

        // === HEADER ===
        renderer.move_cursor(0, 0)?;
        let title = match viewer.mode {
            ViewerMode::Browser => format!(" Media Viewer - {} ", viewer.current_dir.display()),
            ViewerMode::ImageView => " Image Viewer ".to_string(),
            ViewerMode::VideoView => " Video Player ".to_string(),
        };
        let title_truncated = if title.len() > cols as usize - 2 {
            format!("{}...", &title[..cols as usize - 5])
        } else {
            title
        };
        let padding = " ".repeat((cols as usize).saturating_sub(title_truncated.len()));
        renderer.write_styled(&format!("{}{}", title_truncated, padding), "\x1b[1;97;44m")?;

        match viewer.mode {
            ViewerMode::Browser => {
                // File browser view
                let list_start = 2u16;
                let list_height = rows.saturating_sub(4) as usize;

                // Ensure selected is visible
                if viewer.selected_idx < viewer.scroll_offset {
                    viewer.scroll_offset = viewer.selected_idx;
                } else if viewer.selected_idx >= viewer.scroll_offset + list_height {
                    viewer.scroll_offset = viewer.selected_idx - list_height + 1;
                }

                // Render file list
                for (i, entry) in viewer
                    .entries
                    .iter()
                    .skip(viewer.scroll_offset)
                    .take(list_height)
                    .enumerate()
                {
                    let row = list_start + i as u16;
                    renderer.move_cursor(0, row)?;

                    let actual_idx = viewer.scroll_offset + i;
                    let is_selected = actual_idx == viewer.selected_idx;

                    // Icon
                    let icon = if entry.is_dir {
                        " "
                    } else if entry.is_image {
                        " "
                    } else if entry.is_video {
                        " "
                    } else {
                        " "
                    };

                    // Format entry
                    let name_width = (cols as usize).saturating_sub(4);
                    let display_name = if entry.name.len() > name_width {
                        format!("{}...", &entry.name[..name_width - 3])
                    } else {
                        format!("{:<width$}", entry.name, width = name_width)
                    };

                    let line = format!(" {}{}", icon, display_name);

                    if is_selected {
                        renderer.write_styled(&line, "\x1b[1;97;46m")?;
                    } else if entry.is_dir {
                        renderer.write_styled(&line, "\x1b[1;34m")?;
                    } else if entry.is_image {
                        renderer.write_styled(&line, "\x1b[32m")?;
                    } else if entry.is_video {
                        renderer.write_styled(&line, "\x1b[35m")?;
                    } else {
                        renderer.write_text(&line)?;
                    }
                }

                // Controls hint
                renderer.move_cursor(0, rows - 2)?;
                renderer.write_styled(" [Enter] Open  [Backspace] Back  [q] Quit ", "\x1b[2m")?;
            }

            ViewerMode::ImageView | ViewerMode::VideoView => {
                // Image/Video view
                let content_start = 2u16;
                let content_height = rows.saturating_sub(4);
                let content_width = cols;

                // Get image data - let Kitty's cell-based placement handle scaling
                // This avoids the hardcoded pixel assumptions in geometry detection
                let image_data = match &viewer.content {
                    MediaContent::Image(img) => {
                        // For zoomed/panned view, crop the image; for fit mode, send original
                        let (img_w, img_h) = img.dimensions();

                        if viewer.fit_mode {
                            // Send original image, let Kitty scale via c/r parameters
                            let rgb = img.to_rgb8();
                            Some((rgb.into_raw(), img_w, img_h))
                        } else {
                            // Crop for zoom/pan
                            let view_w = (img_w as f32 / viewer.zoom) as u32;
                            let view_h = (img_h as f32 / viewer.zoom) as u32;
                            let center_x = img_w as i32 / 2;
                            let center_y = img_h as i32 / 2;
                            let pan_scale = 1.0 / viewer.zoom;
                            let src_x = (center_x - (view_w as i32 / 2)
                                + (viewer.pan_x as f32 * pan_scale) as i32)
                                .max(0)
                                .min((img_w - view_w.min(img_w)) as i32)
                                as u32;
                            let src_y = (center_y - (view_h as i32 / 2)
                                + (viewer.pan_y as f32 * pan_scale) as i32)
                                .max(0)
                                .min((img_h - view_h.min(img_h)) as i32)
                                as u32;
                            let crop_w = view_w.min(img_w - src_x);
                            let crop_h = view_h.min(img_h - src_y);
                            let cropped = img.crop_imm(src_x, src_y, crop_w, crop_h);
                            let rgb = cropped.to_rgb8();
                            let (cw, ch) = rgb.dimensions();
                            Some((rgb.into_raw(), cw, ch))
                        }
                    }
                    MediaContent::AnimatedGif(player) => {
                        let img = player.current_image();
                        let (img_w, img_h) = img.dimensions();
                        // Send original frame, let Kitty scale
                        let rgb = img.to_rgb8();
                        Some((rgb.into_raw(), img_w, img_h))
                    }
                    MediaContent::Video(player) => {
                        // Extract frame at native resolution
                        let need_extract = match &viewer.cached_frame {
                            Some((frame_num, _, _, _)) => *frame_num != player.current_frame,
                            None => true,
                        };

                        if need_extract {
                            // Extract at 1920x1080 max to avoid huge frames
                            if let Ok(img) = player.extract_frame(player.current_frame, 1920, 1080)
                            {
                                let rgb = img.to_rgb8();
                                let (w, h) = rgb.dimensions();
                                viewer.cached_frame =
                                    Some((player.current_frame, rgb.into_raw(), w, h));
                            }
                        }

                        viewer
                            .cached_frame
                            .as_ref()
                            .map(|(_, data, w, h)| (data.clone(), *w, *h))
                    }
                    MediaContent::None => None,
                };

                // Render the image
                if let Some((data, w, h)) = image_data {
                    renderer.render_image(
                        &data,
                        w,
                        h,
                        0,
                        content_start,
                        Some(content_width),
                        Some(content_height),
                    )?;
                }

                // Playback info for video/gif
                let info = match &viewer.content {
                    MediaContent::AnimatedGif(player) => {
                        format!(
                            "Frame {}/{} | {}",
                            player.current_frame + 1,
                            player.frames.len(),
                            if player.playing { "Playing" } else { "Paused" }
                        )
                    }
                    MediaContent::Video(player) => {
                        let time = player.current_frame as f32 / player.fps;
                        format!(
                            "Frame {}/{} | {:.1}s/{:.1}s | {}",
                            player.current_frame + 1,
                            player.total_frames,
                            time,
                            player.duration_secs,
                            if player.playing { "Playing" } else { "Paused" }
                        )
                    }
                    _ => String::new(),
                };

                if !info.is_empty() {
                    renderer.move_cursor(0, 1)?;
                    renderer.write_styled(&format!(" {} ", info), "\x1b[2m")?;
                }

                // Controls
                renderer.move_cursor(0, rows - 2)?;
                let controls = match viewer.mode {
                    ViewerMode::ImageView => " [hjkl] Pan  [+/-] Zoom  [w/b] Next/Prev file  [f] Fit  [Space] Play  [Esc] Back ",
                    ViewerMode::VideoView => " [hjkl] Pan  [w/b] Next/Prev  [Space] Play  [/]] Frame  [Esc] Back ",
                    _ => "",
                };
                renderer.write_styled(controls, "\x1b[2m")?;
            }
        }

        // === STATUS BAR ===
        renderer.move_cursor(0, rows - 1)?;
        let status_left = format!(" {} | {} ", backend.name(), viewer.status);
        let status_right = match viewer.mode {
            ViewerMode::Browser => {
                format!(" {}/{} ", viewer.selected_idx + 1, viewer.entries.len())
            }
            ViewerMode::ImageView | ViewerMode::VideoView => {
                format!(" Zoom: {:.0}% ", viewer.zoom * 100.0)
            }
        };
        let status_mid_width =
            (cols as usize).saturating_sub(status_left.len() + status_right.len());
        let status_mid = format!("{:^width$}", "", width = status_mid_width);
        renderer.write_styled(
            &format!("{}{}{}", status_left, status_mid, status_right),
            "\x1b[1;97;45m",
        )?;

        renderer.end_frame()?;

        // Frame timing
        let frame_time = loop_start.elapsed();
        let target = Duration::from_millis(16); // ~60fps
        if frame_time < target {
            std::thread::sleep(target - frame_time);
        }
    }

    renderer.clear_images()?;
    renderer.exit_alt_screen()?;
    renderer.show_cursor()?;

    println!("Media viewer closed.");
    Ok(())
}
