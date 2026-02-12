//! Graphics backend demo - Tests image rendering with scrolling waveform
//!
//! Demonstrates the Animation component for rendering animated content
//! in a full-screen layout between a header and status bar.
//!
//! Controls:
//! - 'q' or Esc: Quit
//! - 'u': Toggle Unicode placeholder mode
//! - 'a' or Space: Toggle animation (play/pause)
//! - 'c': Clear all images

use anyhow::Result;
use mkui::{
    event::{Event, EventPoller, Key},
    Animation, Renderer,
};
use std::time::{Duration, Instant};

fn main() -> Result<()> {
    eprintln!("=== mkui Graphics Demo ===");
    eprintln!("TMUX: {:?}", std::env::var("TMUX").ok());
    eprintln!("TERM: {:?}", std::env::var("TERM").ok());

    let mut renderer = Renderer::new()?;
    let backend = renderer.graphics_backend();
    let in_tmux = renderer.in_multiplexer();

    eprintln!("Backend: {}, In tmux: {}", backend.name(), in_tmux);

    renderer.enter_alt_screen()?;
    renderer.hide_cursor()?;

    let events = EventPoller::new()?;

    // State
    let start_time = Instant::now();
    // Use renderer's auto-detection: placeholder mode in tmux, direct mode otherwise.
    let mut unicode_placeholders = in_tmux;

    eprintln!(
        "Unicode placeholders: {} (press 'u' to toggle)",
        unicode_placeholders
    );

    // Create the Animation component with pixel dimensions
    let (img_width, img_height) = (400u32, 120u32);
    let mut animation = Animation::new(img_width, img_height);

    let mut frame_num = 0u32;
    loop {
        frame_num += 1;
        let loop_start = Instant::now();

        // Handle events
        if let Some(event) = events.poll(Duration::from_millis(1))? {
            match event {
                Event::Key(Key::Char('q')) | Event::Key(Key::Ctrl('c')) | Event::Key(Key::Esc) => {
                    break;
                }
                Event::Key(Key::Char('u')) => {
                    unicode_placeholders = !unicode_placeholders;
                    renderer.set_unicode_placeholders(unicode_placeholders);
                    renderer.clear_images()?;
                    eprintln!("Unicode placeholders: {}", unicode_placeholders);
                }
                Event::Key(Key::Char('a')) | Event::Key(Key::Char(' ')) => {
                    animation.toggle();
                    eprintln!(
                        "Animation: {}",
                        if animation.is_playing() {
                            "playing"
                        } else {
                            "paused"
                        }
                    );
                }
                Event::Key(Key::Char('c')) => {
                    renderer.clear_images()?;
                    eprintln!("Cleared all images");
                }
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

        // Get elapsed time for animation
        let elapsed = if animation.is_playing() {
            start_time.elapsed().as_secs_f32()
        } else {
            0.0
        };

        let (cols, rows) = renderer.context().char_dimensions();

        // Begin frame - don't clear graphics since we're using animation ID reuse
        renderer.begin_frame_with_options(false)?;
        renderer.clear()?;

        // === HEADER (row 0) ===
        renderer.move_cursor(0, 0)?;
        let header_text = format!(" mkui Animation Demo - {} ", backend.name());
        let header_padding = " ".repeat(cols as usize - header_text.len());
        renderer.write_styled(
            &format!("{}{}", header_text, header_padding),
            "\x1b[1;97;44m",
        )?;

        // === INFO BAR (row 2) ===
        renderer.move_cursor(0, 2)?;
        let mode_str = if unicode_placeholders {
            "Unicode Placeholders"
        } else {
            "Direct"
        };
        let status_str = if animation.is_playing() {
            "Playing"
        } else {
            "Paused"
        };
        renderer.write_text(&format!(
            "{} | {}x{} | Mode: {} | tmux: {} | {:.1}s | Frame #{}",
            status_str,
            cols,
            rows,
            mode_str,
            if in_tmux { "yes" } else { "no" },
            elapsed,
            frame_num,
        ))?;

        // === CONTROLS (row 3) ===
        renderer.move_cursor(0, 3)?;
        renderer.write_styled("[q]uit  [space]play/pause  [u]nicode  [c]lear", "\x1b[2m")?;

        // === ANIMATION AREA (rows 5 to rows-2) ===
        // Calculate animation bounds - full width, between controls and status bar
        let anim_start_row = 5u16;
        let status_bar_row = rows.saturating_sub(1);
        let available_height = status_bar_row.saturating_sub(anim_start_row);

        // Animation cell dimensions
        let anim_col = 2u16;
        let anim_width_cells = cols.saturating_sub(4).min(80).max(10);
        let anim_height_cells = available_height.min(15).max(3);

        // Generate the current animation frame
        if animation.is_playing() || frame_num == 1 {
            let frame_data = render_waveform_view(img_width, img_height, elapsed);
            animation.set_frame(frame_data);
        }

        // Render the animation
        if anim_width_cells > 4 && anim_height_cells >= 3 {
            renderer.render_image(
                animation.frame_buffer_mut(),
                img_width,
                img_height,
                anim_col,
                anim_start_row,
                Some(anim_width_cells),
                Some(anim_height_cells),
            )?;
        }

        // === STATUS BAR (last row) ===
        renderer.move_cursor(0, status_bar_row)?;
        let status_left = format!(" mkui v0.1 | {} ", backend.name());
        let status_right = format!(" {:.1}s ", elapsed);
        let status_center_width = cols as usize - status_left.len() - status_right.len();
        let status_center = format!(
            "{:^width$}",
            if animation.is_playing() {
                "Playing"
            } else {
                "Paused"
            },
            width = status_center_width
        );
        renderer.write_styled(
            &format!("{}{}{}", status_left, status_center, status_right),
            "\x1b[1;97;45m",
        )?;

        renderer.end_frame()?;

        // Target ~30 FPS
        let frame_time = loop_start.elapsed();
        let target = Duration::from_millis(33);
        if frame_time < target {
            std::thread::sleep(target - frame_time);
        }
    }

    renderer.clear_images()?;
    renderer.exit_alt_screen()?;
    renderer.show_cursor()?;

    println!("Demo finished.");
    Ok(())
}

/// Render audio track waveform - DAW style visualization
fn render_waveform_view(width: u32, height: u32, time: f32) -> Vec<u8> {
    let mut data = vec![0u8; (width * height * 3) as usize];
    let mid_y = height as f32 / 2.0;

    // Dark background with subtle gradient
    for y in 0..height {
        for x in 0..width {
            let idx = ((y * width + x) * 3) as usize;
            let vert_grad = (y as f32 / height as f32 - 0.5).abs() * 0.3;
            data[idx] = (18.0 + vert_grad * 10.0) as u8;
            data[idx + 1] = (20.0 + vert_grad * 10.0) as u8;
            data[idx + 2] = (28.0 + vert_grad * 15.0) as u8;
        }
    }

    // Draw center line
    for x in 0..width {
        let idx = ((mid_y as u32 * width + x) * 3) as usize;
        data[idx] = 35;
        data[idx + 1] = 40;
        data[idx + 2] = 50;
    }

    // Playhead position (moves through the track)
    let playhead = (time * 0.1).fract(); // Position 0-1 in track
    let playhead_x = (playhead * width as f32) as u32;

    // Generate realistic audio waveform amplitudes
    for x in 0..width {
        let t = x as f32 / width as f32;

        // Simulate different sections of a song
        let section = (t * 4.0).floor(); // 4 sections

        // Base amplitude varies by section (verse quieter, chorus louder)
        let section_amp = match section as i32 % 4 {
            0 => 0.5, // Intro - medium
            1 => 0.8, // Verse - fuller
            2 => 1.0, // Chorus - loud
            3 => 0.6, // Bridge - medium
            _ => 0.7,
        };

        // Smooth transitions between sections
        let section_pos = (t * 4.0).fract();
        let transition = if section_pos < 0.1 {
            section_pos / 0.1
        } else if section_pos > 0.9 {
            1.0 - (section_pos - 0.9) / 0.1
        } else {
            1.0
        };

        // Simulate transients (drum hits, percussive elements)
        let beat_pos = (t * 32.0).fract(); // 32 beats in view
        let transient = if beat_pos < 0.05 {
            1.0 + (1.0 - beat_pos / 0.05) * 0.4 // Sharp attack
        } else if beat_pos < 0.15 {
            1.0 + (1.0 - (beat_pos - 0.05) / 0.1) * 0.2 // Quick decay
        } else {
            1.0
        };

        // Sub-transients for hi-hats/details
        let sub_beat = (t * 64.0).fract();
        let sub_transient = if sub_beat < 0.03 { 1.15 } else { 1.0 };

        // High frequency detail (texture)
        let detail1 = ((t * 500.0 + time * 50.0).sin() * 0.15).abs();
        let detail2 = ((t * 1234.5 + time * 30.0).sin() * 0.1).abs();
        let detail3 = ((t * 2847.3).sin() * 0.08).abs();

        // Mid frequency content
        let mid1 = ((t * 80.0 + time * 5.0).sin() * 0.3).abs();
        let mid2 = ((t * 120.0 + time * 3.0).sin() * 0.2).abs();

        // Low frequency (bass) - slower movement
        let bass = ((t * 20.0 + time * 2.0).sin() * 0.25).abs();

        // Combine all elements
        let raw_amplitude = (bass + mid1 + mid2 + detail1 + detail2 + detail3)
            * section_amp
            * transition
            * transient
            * sub_transient;

        // Add slight randomness for natural feel
        let noise = ((x as f32 * 9876.54 + time * 100.0).sin() * 0.05).abs();

        let amplitude = (raw_amplitude + noise).min(1.0);

        // Calculate wave height (symmetric around center)
        let wave_height = (amplitude * mid_y * 0.9) as i32;

        // Determine color based on position relative to playhead
        let is_played = x < playhead_x;

        // Draw the waveform bar (filled from center)
        for dy in -wave_height..=wave_height {
            let y = (mid_y as i32 + dy).clamp(0, height as i32 - 1) as u32;
            let idx = ((y * width + x) * 3) as usize;

            // Color gradient based on amplitude and played status
            let intensity = 1.0 - (dy.abs() as f32 / wave_height.max(1) as f32) * 0.3;

            if is_played {
                // Played portion - cyan/teal, brighter
                data[idx] = (70.0 * intensity) as u8;
                data[idx + 1] = (200.0 * intensity) as u8;
                data[idx + 2] = (230.0 * intensity) as u8;
            } else {
                // Unplayed portion - dimmer blue-gray
                data[idx] = (50.0 * intensity) as u8;
                data[idx + 1] = (80.0 * intensity) as u8;
                data[idx + 2] = (120.0 * intensity) as u8;
            }
        }

        // Add peaks highlight (brightest point at the tips)
        if wave_height > 2 {
            for &tip_dy in &[-wave_height, wave_height] {
                let tip_y = (mid_y as i32 + tip_dy).clamp(0, height as i32 - 1) as u32;
                let idx = ((tip_y * width + x) * 3) as usize;
                if is_played {
                    data[idx] = data[idx].saturating_add(40);
                    data[idx + 1] = data[idx + 1].saturating_add(55);
                    data[idx + 2] = 255;
                } else {
                    data[idx] = data[idx].saturating_add(20);
                    data[idx + 1] = data[idx + 1].saturating_add(30);
                    data[idx + 2] = data[idx + 2].saturating_add(40);
                }
            }
        }
    }

    // Draw playhead line
    if playhead_x < width {
        for y in 0..height {
            let idx = ((y * width + playhead_x) * 3) as usize;
            data[idx] = 255;
            data[idx + 1] = 255;
            data[idx + 2] = 255;
        }
        // Glow around playhead
        for dx in 1..=2i32 {
            for &px in &[
                playhead_x.saturating_sub(dx as u32),
                (playhead_x + dx as u32).min(width - 1),
            ] {
                for y in 0..height {
                    let idx = ((y * width + px) * 3) as usize;
                    let glow = 0.5 / dx as f32;
                    data[idx] = (data[idx] as f32 + 100.0 * glow).min(255.0) as u8;
                    data[idx + 1] = (data[idx + 1] as f32 + 100.0 * glow).min(255.0) as u8;
                    data[idx + 2] = (data[idx + 2] as f32 + 100.0 * glow).min(255.0) as u8;
                }
            }
        }
    }

    data
}
