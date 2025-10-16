use macroquad::prelude::*;
use std::path::PathBuf;
use std::time::SystemTime;
use std::fs;

/// Show a combined save picker: allows entering a new filename or selecting an existing
/// snapshot to overwrite. Returns Some(path_string) when user confirms a save path, or None on cancel.
pub async fn pick_save(initial_name: Option<&str>) -> Option<String> {
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

    // Sort by modified time desc
    files.sort_by(|a, b| {
        let ma = a.metadata().and_then(|m| m.modified()).unwrap_or(SystemTime::UNIX_EPOCH);
        let mb = b.metadata().and_then(|m| m.modified()).unwrap_or(SystemTime::UNIX_EPOCH);
        mb.cmp(&ma)
    });

    let mut input = initial_name.unwrap_or("").to_string();
    let page_size: usize = 10;
    let mut page: usize = 0;
    let mut confirm_delete: Option<PathBuf> = None;
    let mut toast: Option<(String, f32)> = None; // (message, remaining_secs)

    loop {
        clear_background(Color::new(0.05, 0.05, 0.08, 1.0));
        let w = screen_width();
        let h = screen_height();

        // Title
        draw_text("Save Simulation (enter name or pick file to overwrite)", w * 0.5 - 360.0, h * 0.12, 28.0, WHITE);

        // Input box for new name
        let ib_x = w * 0.12;
        let ib_w = w * 0.76;
        let ib_y = h * 0.18;
        let ib_h = 40.0;
        draw_rectangle(ib_x, ib_y, ib_w, ib_h, Color::new(0.12, 0.12, 0.14, 1.0));
        draw_rectangle_lines(ib_x, ib_y, ib_w, ib_h, 2.0, WHITE);
        let display = if input.is_empty() { "filename".to_string() } else { input.clone() };
        draw_text(&format!("{}", display), ib_x + 8.0, ib_y + 28.0, 24.0, YELLOW);

        // Buttons: Save (left) Cancel (right)
        let btn_w = 160.0;
        let btn_h = 40.0;
        let save_x = ib_x;
        let save_y = ib_y + ib_h + 8.0;
        let cancel_x = ib_x + ib_w - btn_w;
        let cancel_y = save_y;

    // Draw list of existing files below
        let list_x = ib_x;
        let list_y = save_y + btn_h + 18.0;
        let list_w = ib_w;
        let item_h = 36.0;
        let start = page * page_size;
        let end = ((page + 1) * page_size).min(files.len());

        // Draw list background
        draw_rectangle(list_x, list_y, list_w, (item_h + 8.0) * (end - start) as f32 + 8.0, Color::new(0.14, 0.14, 0.16, 1.0));

        for (i, p) in files[start..end].iter().enumerate() {
            let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("?");
            let rect_x = list_x + 8.0;
            let rect_y = list_y + 8.0 + (i as f32) * (item_h + 8.0);
            let rect_w = list_w - 16.0;
            let rect_h = item_h;
            let (mx, my) = mouse_position();
            let hovered = mx >= rect_x && mx <= rect_x + rect_w && my >= rect_y && my <= rect_y + rect_h;
            let bg = if hovered { Color::new(0.25, 0.55, 0.9, 1.0) } else { Color::new(0.18, 0.18, 0.18, 1.0) };
            draw_rectangle(rect_x, rect_y, rect_w, rect_h, bg);
            draw_rectangle_lines(rect_x, rect_y, rect_w, rect_h, 2.0, BLACK);
            draw_text(name, rect_x + 8.0, rect_y + 24.0, 20.0, BLACK);

            // Delete button on the right of the row
            let del_w = 30.0;
            let del_h = 24.0;
            let del_x = rect_x + rect_w - del_w - 8.0;
            let del_y = rect_y + (rect_h - del_h) * 0.5;
            let del_hovered = mx >= del_x && mx <= del_x + del_w && my >= del_y && my <= del_y + del_h;
            let del_bg = if del_hovered { Color::new(0.8, 0.2, 0.2, 1.0) } else { Color::new(0.6, 0.15, 0.15, 1.0) };
            draw_rectangle(del_x, del_y, del_w, del_h, del_bg);
            draw_rectangle_lines(del_x, del_y, del_w, del_h, 2.0, BLACK);
            draw_text("X", del_x + 9.0, del_y + 18.0, 18.0, WHITE);

            // If hovered and clicked, return that path (overwrite)
            if is_mouse_button_pressed(MouseButton::Left) {
                if del_hovered {
                    // Ask for confirmation
                    confirm_delete = Some(p.clone());
                } else if hovered && confirm_delete.is_none() {
                    return Some(p.to_string_lossy().into_owned());
                }
            }
        }

        // Pagination and controls
        let controls_y = list_y + (item_h + 8.0) * (end - start) as f32 + 22.0;
        let cx = w * 0.5 - 160.0;
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
        let nx = w * 0.5 - 0.0;
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
        draw_rectangle(cancel_x, cancel_y, btn_w, btn_h, Color::new(0.5, 0.2, 0.2, 1.0));
        draw_rectangle_lines(cancel_x, cancel_y, btn_w, btn_h, 2.0, BLACK);
        draw_text("Cancel", cancel_x + 30.0, cancel_y + 26.0, 24.0, BLACK);
        if is_mouse_button_pressed(MouseButton::Left) {
            let (mx, my) = mouse_position();
            if mx >= cancel_x && mx <= cancel_x + btn_w && my >= cancel_y && my <= cancel_y + btn_h { return None; }
        }

        // Save button (uses input string)
        draw_rectangle(save_x, save_y, btn_w, btn_h, Color::new(0.2, 0.6, 0.2, 1.0));
        draw_rectangle_lines(save_x, save_y, btn_w, btn_h, 2.0, WHITE);
        draw_text("Save", save_x + 56.0, save_y + 26.0, 24.0, WHITE);
        if is_mouse_button_pressed(MouseButton::Left) {
            let (mx, my) = mouse_position();
            if mx >= save_x && mx <= save_x + btn_w && my >= save_y && my <= save_y + btn_h {
                if !input.trim().is_empty() {
                    let filename = format!("snapshots/{}.json", input.trim());
                    return Some(filename);
                }
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
        } else {
            // Keyboard input handling (only when not in modal)
            if is_key_pressed(KeyCode::Backspace) { input.pop(); }
            if is_key_pressed(KeyCode::Escape) { return None; }
            if is_key_pressed(KeyCode::Enter) {
                if !input.trim().is_empty() {
                    let filename = format!("snapshots/{}.json", input.trim());
                    return Some(filename);
                }
            }
            while let Some(ch) = get_char_pressed() {
                if ch.is_ascii() && (ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' || ch == '.') {
                    input.push(ch);
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
