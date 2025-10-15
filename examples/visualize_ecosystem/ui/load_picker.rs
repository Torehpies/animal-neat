use macroquad::prelude::*;
use std::path::PathBuf;
use std::time::SystemTime;

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
    if files.is_empty() {
        // Inform the user and return None on any key/click
        loop {
            clear_background(Color::new(0.05, 0.05, 0.08, 1.0));
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

    loop {
        clear_background(Color::new(0.05, 0.05, 0.08, 1.0));
        let w = screen_width();
        let h = screen_height();

        draw_text("Select snapshot to load", w * 0.5 - 180.0, h * 0.12, 36.0, WHITE);

        let list_x = w * 0.15;
        let list_w = w * 0.7;
        let mut list_y = h * 0.18;
        let item_h = 36.0;

        let start = page * page_size;
        let end = ((page + 1) * page_size).min(files.len());
        for (i, p) in files[start..end].iter().enumerate() {
            let idx = start + i;
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

            if hovered && is_mouse_button_pressed(MouseButton::Left) {
                return Some(p.to_string_lossy().into_owned());
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
            if mx >= cancel_x && mx <= cancel_x + btn_w && my >= controls_y && my <= controls_y + btn_h { return None; }
        }

        next_frame().await;
    }
}
