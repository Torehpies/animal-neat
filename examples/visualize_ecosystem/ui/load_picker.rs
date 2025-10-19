use macroquad::prelude::*;
use std::path::PathBuf;
use std::time::SystemTime;
use std::fs;

use crate::ui_main_menu::particle_system::ParticleSystem;

/// Show a mouse-driven snapshot picker for files in `snapshots/`.
/// Returns Some(path_string) when the user clicks a file, or None when cancelled.
pub async fn pick_snapshot() -> Option<String> {
    // Collect files
    let mut files: Vec<PathBuf> = Vec::new();
    if let Ok(entries) = std::fs::read_dir("snapshots") {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.extension().and_then(|s| s.to_str()) == Some("json") {
                files.push(p);
            }
        }
    }
    
    let mut particle_system = ParticleSystem::new(100);
    
    if files.is_empty() {
        // Inform the user and return None on any key/click
        loop {
            clear_background(Color::new(0.05, 0.05, 0.08, 1.0));
            particle_system.update();
            particle_system.draw();
            let w = screen_width();
            let h = screen_height();
            draw_text("No snapshots found in snapshots/", w * 0.5 - 220.0, h * 0.5, 28.0, WHITE);
            draw_text("Press Esc or click to continue", w * 0.5 - 160.0, h * 0.5 + 40.0, 20.0, WHITE);
            if is_key_pressed(KeyCode::Escape) || is_mouse_button_pressed(MouseButton::Left) { return None; }
            next_frame().await;
        }
    }

    // Sort by modified time desc (newest first)
    files.sort_by(|a, b| {
        let ma = a.metadata().and_then(|m| m.modified()).unwrap_or(SystemTime::UNIX_EPOCH);
        let mb = b.metadata().and_then(|m| m.modified()).unwrap_or(SystemTime::UNIX_EPOCH);
        mb.cmp(&ma)
    });


    let page_size: usize = 10;
    let mut page: usize = 0;
    let mut confirm_delete: Option<PathBuf> = None;
    let mut toast: Option<(String, f32)> = None; // (message, remaining_secs)

    loop {
        clear_background(Color::new(0.05, 0.05, 0.08, 1.0));
        particle_system.update();
        particle_system.draw();
        
        let w = screen_width();
        let h = screen_height();

        draw_text("Select snapshot to load", w * 0.5 - 180.0, h * 0.12, 36.0, WHITE);

        let list_x = w * 0.15;
        let list_w = w * 0.7;
        let list_y = h * 0.18;
        let item_h = 36.0;

        let start = page * page_size;
        let end = ((page + 1) * page_size).min(files.len());
        for (i, p) in files[start..end].iter().enumerate() {
            let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("?");
            let rect_x = list_x;
            let rect_y = list_y + (i as f32) * (item_h + 8.0);
            let rect_w = list_w;
            let rect_h = item_h + 6.0;

            // highlight on hover
            let (mx, my) = mouse_position();
            let hovered = mx >= rect_x && mx <= rect_x + rect_w && my >= rect_y && my <= rect_y + rect_h;
            let bg = if hovered { Color::new(0.2, 0.6, 0.9, 1.0) } else { Color::new(0.18, 0.18, 0.18, 1.0) };
            draw_rectangle(rect_x, rect_y, rect_w, rect_h, bg);
            draw_rectangle_lines(rect_x, rect_y, rect_w, rect_h, 2.0, BLACK);
            draw_text(name, rect_x + 8.0, rect_y + 22.0, 20.0, BLACK);

            // Delete button on the right of the row
            let del_w = 30.0;
            let del_h = 24.0;
            let del_x = rect_x + rect_w - del_w - 8.0;
            let del_y = rect_y + (rect_h - del_h) * 0.5;
            let (mx, my) = mouse_position();
            let del_hovered = mx >= del_x && mx <= del_x + del_w && my >= del_y && my <= del_y + del_h;
            let del_bg = if del_hovered { Color::new(0.8, 0.2, 0.2, 1.0) } else { Color::new(0.6, 0.15, 0.15, 1.0) };
            draw_rectangle(del_x, del_y, del_w, del_h, del_bg);
            draw_rectangle_lines(del_x, del_y, del_w, del_h, 2.0, BLACK);
            draw_text("X", del_x + 9.0, del_y + 18.0, 18.0, WHITE);

            if is_mouse_button_pressed(MouseButton::Left) {
                if del_hovered {
                    confirm_delete = Some(p.clone());
                } else if hovered && confirm_delete.is_none() {
                    // File selected - play effect and return
                    let target_center = vec2(rect_x + rect_w / 2.0, rect_y + rect_h / 2.0);
                    particle_system.activate_pull(target_center, 0.5);

                    let effect_duration = 0.8;
                    let start_time = get_time();

                    loop {
                        if get_time() - start_time >= effect_duration {
                            break;
                        }
                        
                        let t = ((get_time() - start_time) / effect_duration).clamp(0.0, 1.0) as f32;

                        clear_background(Color::new(0.05, 0.05, 0.08, 1.0));
                        particle_system.update();
                        particle_system.draw();

                        // Draw picker UI fading out
                        draw_text("Select snapshot to load", w * 0.5 - 180.0, h * 0.12, 36.0, Color::new(1.0, 1.0, 1.0, 1.0 - t * 0.8));

                        // Fade overlay on top
                        draw_rectangle(
                            0.0,
                            0.0,
                            screen_width(),
                            screen_height(),
                            Color::new(0.0, 0.0, 0.0, t * 0.8),
                        );

                        next_frame().await;
                    }
                    
                    return Some(p.to_string_lossy().into_owned());
                }
            }
        }

        // Pagination and controls
        let controls_y = h * 0.88;
        let btn_w = 140.0;
        let btn_h = 40.0;
        let cx = w * 0.5 - btn_w * 1.5 - 10.0;
        // Prev
        if page > 0 {
            draw_rectangle(cx, controls_y, btn_w, btn_h, Color::new(0.3, 0.3, 0.3, 1.0));
            draw_rectangle_lines(cx, controls_y, btn_w, btn_h, 2.0, BLACK);
            draw_text("Prev", cx + 36.0, controls_y + 26.0, 24.0, BLACK);
            if is_mouse_button_pressed(MouseButton::Left) {
                let (mx, my) = mouse_position();
                if mx >= cx && mx <= cx + btn_w && my >= controls_y && my <= controls_y + btn_h { page = page.saturating_sub(1); }
            }
        }
        // Next
        let nx = w * 0.5 - btn_w * 0.5 + 10.0;
        if (page + 1) * page_size < files.len() {
            draw_rectangle(nx, controls_y, btn_w, btn_h, Color::new(0.3, 0.3, 0.3, 1.0));
            draw_rectangle_lines(nx, controls_y, btn_w, btn_h, 2.0, BLACK);
            draw_text("Next", nx + 36.0, controls_y + 26.0, 24.0, BLACK);
            if is_mouse_button_pressed(MouseButton::Left) {
                let (mx, my) = mouse_position();
                if mx >= nx && mx <= nx + btn_w && my >= controls_y && my <= controls_y + btn_h { page += 1; }
            }
        }

        // Cancel
        let cancel_x = w * 0.5 + btn_w * 0.5 + 30.0;
        draw_rectangle(cancel_x, controls_y, btn_w, btn_h, Color::new(0.5, 0.2, 0.2, 1.0));
        draw_rectangle_lines(cancel_x, controls_y, btn_w, btn_h, 2.0, BLACK);
        draw_text("Cancel", cancel_x + 30.0, controls_y + 26.0, 24.0, BLACK);
        if is_mouse_button_pressed(MouseButton::Left) {
            let (mx, my) = mouse_position();
            if mx >= cancel_x && mx <= cancel_x + btn_w && my >= controls_y && my <= controls_y + btn_h {
                // Fade out effect
                let fade_duration = 0.3;
                let start_time = get_time();
                while get_time() - start_time < fade_duration {
                    let progress = (get_time() - start_time) / fade_duration;
                    let alpha = 1.0 - progress as f32;
                    
                    particle_system.update();
                    particle_system.draw();
                    
                    draw_rectangle(0.0, 0.0, w, h, Color::new(0.0, 0.0, 0.0, 1.0 - alpha));
                    next_frame().await;
                }
                
                return None;
            }
        }

        // Confirmation modal for deletion
        if let Some(ref target) = confirm_delete {
            // Dim background
            draw_rectangle(0.0, 0.0, w, h, Color::new(0.0, 0.0, 0.0, 0.4));

            // Modal box
            let mw = w * 0.6;
            let mh = 160.0;
            let mx0 = (w - mw) * 0.5;
            let my0 = (h - mh) * 0.5;
            draw_rectangle(mx0, my0, mw, mh, Color::new(0.15, 0.15, 0.18, 1.0));
            draw_rectangle_lines(mx0, my0, mw, mh, 2.0, WHITE);
            let fname = target.file_name().and_then(|s| s.to_str()).unwrap_or("this file");
            draw_text(&format!("Delete {}?", fname), mx0 + 20.0, my0 + 40.0, 28.0, WHITE);
            draw_text("This cannot be undone.", mx0 + 20.0, my0 + 72.0, 20.0, GRAY);

            let b_w = 120.0; let b_h = 40.0; let gap = 16.0;
            let yes_x = mx0 + mw - b_w * 2.0 - gap - 20.0;
            let no_x = mx0 + mw - b_w - 20.0;
            let by = my0 + mh - b_h - 20.0;

            // Yes button
            draw_rectangle(yes_x, by, b_w, b_h, Color::new(0.7, 0.2, 0.2, 1.0));
            draw_rectangle_lines(yes_x, by, b_w, b_h, 2.0, BLACK);
            draw_text("Delete", yes_x + 22.0, by + 26.0, 24.0, BLACK);

            // No button
            draw_rectangle(no_x, by, b_w, b_h, Color::new(0.3, 0.3, 0.3, 1.0));
            draw_rectangle_lines(no_x, by, b_w, b_h, 2.0, BLACK);
            draw_text("Cancel", no_x + 22.0, by + 26.0, 24.0, BLACK);

            if is_mouse_button_pressed(MouseButton::Left) {
                let (mx, my) = mouse_position();
                if mx >= yes_x && mx <= yes_x + b_w && my >= by && my <= by + b_h {
                    match fs::remove_file(&target) {
                        Ok(_) => {
                            let removed_path = target.clone();
                            files.retain(|p| p != &removed_path);
                            // fix pagination if needed
                            let max_page = if files.is_empty() { 0 } else { (files.len() - 1) / page_size };
                            if page > max_page { page = max_page; }
                            toast = Some((format!("Deleted {}", fname), 2.0));
                        }
                        Err(e) => {
                            toast = Some((format!("Failed to delete {}: {}", fname, e), 3.0));
                        }
                    }
                    confirm_delete = None;
                } else if mx >= no_x && mx <= no_x + b_w && my >= by && my <= by + b_h {
                    confirm_delete = None;
                }
            }
        }

        // Toast message (fade out)
        if let Some((ref msg, ref mut secs)) = toast {
            let dt = get_frame_time();
            *secs -= dt;
            let alpha = secs.clamp(0.0, 2.0) / 2.0; // 0..1
            let bg = Color::new(0.0, 0.0, 0.0, 0.6 * alpha);
            let tw = measure_text(msg, None, 24, 1.0).width + 20.0;
            let tx = w * 0.5 - tw * 0.5;
            let ty = h * 0.9;
            draw_rectangle(tx, ty - 28.0, tw, 34.0, bg);
            draw_text(msg, tx + 10.0, ty, 24.0, WHITE);
            if *secs <= 0.0 { toast = None; }
        }

        next_frame().await;
    }
}