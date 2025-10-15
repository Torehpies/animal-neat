use macroquad::prelude::*;

/// Result of the in-simulation modal menu
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimMenuResult {
    Resume,
    BackToMain,
}

/// Draw the sim menu overlay and handle input for one frame.
/// Returns Some(result) when user makes a choice, None to continue showing menu.
pub fn draw_sim_menu(click_cooldown: &mut f32) -> Option<SimMenuResult> {
    // Draw a semi-transparent dark overlay over the frozen simulation
    let w = screen_width();
    let h = screen_height();
    draw_rectangle(0.0, 0.0, w, h, Color::new(0.0, 0.0, 0.0, 0.7));

    // Panel
    let panel_w = 460.0;
    let panel_h = 280.0;
    let cx = w * 0.5;
    let cy = h * 0.5;
    let panel_x = cx - panel_w * 0.5;
    let panel_y = cy - panel_h * 0.5;
    draw_rectangle(panel_x, panel_y, panel_w, panel_h, Color::new(0.06, 0.06, 0.08, 0.96));
    draw_rectangle_lines(panel_x, panel_y, panel_w, panel_h, 2.0, WHITE);

    // Title
    let title = "Paused — Simulation Menu";
    let title_size = 28.0;
    let title_w = measure_text(title, None, title_size as u16, 1.0).width;
    draw_text(title, panel_x + (panel_w - title_w) / 2.0, panel_y + 45.0, title_size, WHITE);

    // Buttons
    let padding = 30.0;
    let mut by = panel_y + 80.0;
    let btn_w = 130.0;
    let btn_h = 48.0;
    let gap = 15.0;

    let (mx, my) = mouse_position();

    let total_btn_w = btn_w * 3.0 + gap * 2.0;
    let start_x = panel_x + (panel_w - total_btn_w) / 2.0;

    // Resume
    let resume_x = start_x;
    let resume_y = by;
    let hovering_resume = mx >= resume_x && mx <= resume_x + btn_w && my >= resume_y && my <= resume_y + btn_h;
    let resume_col = if hovering_resume { Color::new(0.25, 0.7, 0.25, 1.0) } else { Color::new(0.18, 0.5, 0.18, 1.0) };
    draw_rectangle(resume_x, resume_y, btn_w, btn_h, resume_col);
    draw_rectangle_lines(resume_x, resume_y, btn_w, btn_h, 2.0, WHITE);
    let txt = "Resume";
    let txt_size = 22.0;
    let tw = measure_text(txt, None, txt_size as u16, 1.0).width;
    draw_text(txt, resume_x + (btn_w - tw) / 2.0, resume_y + (btn_h + txt_size) / 2.0 - 4.0, txt_size, WHITE);

    // Save
    let save_x = resume_x + btn_w + gap;
    let save_y = by;
    let hovering_save = mx >= save_x && mx <= save_x + btn_w && my >= save_y && my <= save_y + btn_h;
    let save_col = if hovering_save { Color::new(0.25, 0.6, 0.9, 1.0) } else { Color::new(0.15, 0.45, 0.75, 1.0) };
    draw_rectangle(save_x, save_y, btn_w, btn_h, save_col);
    draw_rectangle_lines(save_x, save_y, btn_w, btn_h, 2.0, WHITE);
    let txt = "Save";
    let tw = measure_text(txt, None, txt_size as u16, 1.0).width;
    draw_text(txt, save_x + (btn_w - tw) / 2.0, save_y + (btn_h + txt_size) / 2.0 - 4.0, txt_size, WHITE);

    // Load
    let load_x = save_x + btn_w + gap;
    let load_y = by;
    let hovering_load = mx >= load_x && mx <= load_x + btn_w && my >= load_y && my <= load_y + btn_h;
    let load_col = if hovering_load { Color::new(0.9, 0.6, 0.25, 1.0) } else { Color::new(0.7, 0.45, 0.12, 1.0) };
    draw_rectangle(load_x, load_y, btn_w, btn_h, load_col);
    draw_rectangle_lines(load_x, load_y, btn_w, btn_h, 2.0, WHITE);
    let txt = "Load";
    let tw = measure_text(txt, None, txt_size as u16, 1.0).width;
    draw_text(txt, load_x + (btn_w - tw) / 2.0, load_y + (btn_h + txt_size) / 2.0 - 4.0, txt_size, WHITE);

    by += btn_h + 24.0;

    // Back to Main Menu
    let back_x = panel_x + padding;
    let back_y = by;
    let back_w = panel_w - 2.0 * padding;
    let hovering_back = mx >= back_x && mx <= back_x + back_w && my >= back_y && my <= back_y + btn_h;
    let back_col = if hovering_back { Color::new(0.8, 0.25, 0.25, 1.0) } else { Color::new(0.6, 0.18, 0.18, 1.0) };
    draw_rectangle(back_x, back_y, back_w, btn_h, back_col);
    draw_rectangle_lines(back_x, back_y, back_w, btn_h, 2.0, WHITE);
    let txt = "Back to Main Menu";
    let tw = measure_text(txt, None, txt_size as u16, 1.0).width;
    draw_text(txt, back_x + (back_w - tw) / 2.0, back_y + (btn_h + txt_size) / 2.0 - 4.0, txt_size, WHITE);

    // Input handling
    if is_mouse_button_pressed(MouseButton::Left) && *click_cooldown <= 0.0 {
        if hovering_resume {
            *click_cooldown = 0.15;
            return Some(SimMenuResult::Resume);
        } else if hovering_back {
            *click_cooldown = 0.15;
            return Some(SimMenuResult::BackToMain);
        }
        // Save and Load buttons will be handled by the main loop
        // to avoid async complications in this frame-based function
    }

    // Cooldown decrement
    if *click_cooldown > 0.0 {
        *click_cooldown = (*click_cooldown - get_frame_time()).max(0.0);
    }

    None // Continue showing menu
}
