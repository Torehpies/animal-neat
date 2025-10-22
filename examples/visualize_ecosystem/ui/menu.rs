//! Simple menu for customizing simulation parameters before starting

use macroquad::prelude::*;

// Pull in backend types/logic and re-export so external modules can continue using ui_menu::SimConfig
pub use crate::menu_backend::{SimConfig, MenuState};
use crate::menu_backend::{EditField, apply_field_value, MenuScreen};

// Simple word-wrapping for multi-paragraph help text
fn draw_text_wrapped(text: &str, x: f32, mut y: f32, max_w: f32, font_sz: f32, line_h: f32, color: Color) {
    for para in text.split('\n') {
        let words: Vec<&str> = para.split_whitespace().collect();
        let mut line = String::new();
        for w in words {
            let candidate = if line.is_empty() { w.to_string() } else { format!("{} {}", line, w) };
            let m = measure_text(&candidate, None, font_sz as u16, 1.0);
            if m.width <= max_w {
                line = candidate;
            } else {
                // draw current line and start a new one
                draw_text(&line, x, y, font_sz, color);
                y += line_h;
                line.clear();
                line.push_str(w);
            }
        }
        if !line.is_empty() {
            draw_text(&line, x, y, font_sz, color);
            y += line_h;
        }
        // blank line between paragraphs
        y += line_h * 0.25;
    }
}

fn draw_help_overlay(state: &mut MenuState, panel_x: f32, panel_y: f32, panel_w: f32, panel_h: f32, ui_scale: f32) {
    let scrim = Color::new(0.0, 0.0, 0.0, 0.55);
    draw_rectangle(0.0, 0.0, screen_width(), screen_height(), scrim);

    // Match the background panel size exactly to the menu panel
    let box_w = panel_w;
    let box_h = panel_h;
    let bx = panel_x;
    let by = panel_y;

    draw_rectangle(bx, by, box_w, box_h, Color::new(0.10, 0.11, 0.14, 1.0));
    draw_rectangle_lines(bx, by, box_w, box_h, 3.0, Color::new(0.45, 0.75, 1.0, 1.0));

    let pad = 22.0 * ui_scale;
    let title_sz = 30.0 * ui_scale;
    let text_sz = 18.0 * ui_scale;
    let line_h = text_sz * 1.35;

    draw_text("Help", bx + pad, by + pad + title_sz, title_sz, WHITE);

    let content_x = bx + pad;
    let content_y = by + pad + title_sz + 14.0 * ui_scale;
    let content_w = box_w - pad * 2.0;

    let help_text = 
    "Overview\n\
    - Agents: Simulated creatures that move, eat, communicate, fight and reproduce. The Agents value controls how many exist at the start of each episode. More agents cost more CPU.\n\
    - Rewards vs Penalties: Fitness weights shape evolution. Positive weights reward behaviors, negative weights penalize them. 'Idle Penalty' is applied when an agent does nothing useful.\n\
    - Fitness (what evolves): Each agent accumulates fitness during an episode. After the episode, evolution prefers genomes with higher fitness. You can steer learning by changing the weights.\n\
    Core Concepts\n\
    Environment\n\
    - World Size: Larger worlds spread agents and reduce encounters; smaller worlds increase interactions.\n\
    - Max Food & Spawn Rate: Upper bound on concurrent plants and probability of new plant spawn each step. Higher food supports bigger populations.\n\
    - Energy: Initial Energy is starting fuel; Max Energy is the cap; Drain/Step is metabolism cost each tick.\n\
    Tips\n\
    - Start with modest weights. If behavior is chaotic, lower Attack/Kill and raise Plant or Lifetime slightly.\n\
    - Use small nudges: 0.05-0.5 often suffices for shaping.\n\
    - Large populations or worlds will reduce FPS; adjust to your machine.";

    // Viewport for help content so it never overflows outside the panel
    let hint_sz = text_sz * 0.95;
    let viewport_y = content_y;
    let viewport_h = (by + box_h) - viewport_y - (pad * 0.9 + hint_sz);

    // Scroll handling when mouse is over help content viewport
    let (mx, my) = mouse_position();
    if mx >= content_x && mx <= content_x + content_w && my >= viewport_y && my <= viewport_y + viewport_h {
        let (_wx, wy) = mouse_wheel();
        if wy.abs() > 0.0 { state.help_scroll -= wy * (40.0 * ui_scale); }
    }

    // Measure total content height for scroll clamping
    let mut total_h = 0.0f32;
    for para in help_text.split('\n') {
        let words: Vec<&str> = para.split_whitespace().collect();
        let mut line = String::new();
        for w in words {
            let candidate = if line.is_empty() { w.to_string() } else { format!("{} {}", line, w) };
            let m = measure_text(&candidate, None, text_sz as u16, 1.0);
            if m.width <= content_w { line = candidate; }
            else { total_h += line_h; line = w.to_string(); }
        }
        if !line.is_empty() { total_h += line_h; }
        total_h += line_h * 0.25;
    }
    let max_scroll = (total_h - viewport_h).max(0.0);
    if state.help_scroll < 0.0 { state.help_scroll = 0.0; }
    if state.help_scroll > max_scroll { state.help_scroll = max_scroll; }

    // Draw wrapped text but cull lines outside the viewport rectangle, with scroll offset applied
    {
        let mut y_line = content_y;
        for para in help_text.split('\n') {
            let words: Vec<&str> = para.split_whitespace().collect();
            let mut line = String::new();
            for w in words {
                let candidate = if line.is_empty() { w.to_string() } else { format!("{} {}", line, w) };
                let m = measure_text(&candidate, None, text_sz as u16, 1.0);
                if m.width <= content_w {
                    line = candidate;
                } else {
                    // Draw this line if within viewport
                    let draw_y = y_line - state.help_scroll;
                    if draw_y + 2.0 >= viewport_y && draw_y - line_h <= viewport_y + viewport_h {
                        draw_text(&line, content_x, draw_y, text_sz, LIGHTGRAY);
                    }
                    y_line += line_h;
                    line.clear();
                    line.push_str(w);
                }
            }
            if !line.is_empty() {
                let draw_y = y_line - state.help_scroll;
                if draw_y + 2.0 >= viewport_y && draw_y - line_h <= viewport_y + viewport_h {
                    draw_text(&line, content_x, draw_y, text_sz, LIGHTGRAY);
                }
                y_line += line_h;
            }
            // paragraph spacing
            y_line += line_h * 0.25;
            if (y_line - state.help_scroll) > viewport_y + viewport_h + line_h { break; }
        }
    }

    // Close hint at the bottom
    let hint = "Click anywhere to close this help";
    let m = measure_text(hint, None, (hint_sz) as u16, 1.0);
    draw_text(
        hint,
        bx + (box_w - m.width) / 2.0,
        by + box_h - pad * 0.6,
        hint_sz,
        Color::new(0.75, 0.85, 1.0, 0.9),
    );
}

// --- Small UI helpers for clearer, discoverable editing ---
fn field_step(field: EditField) -> f32 {
    match field {
        EditField::PopSize | EditField::MaxFood => 10.0,
        EditField::Herbivores | EditField::Carnivores => 1.0,
        EditField::FoodRespawnRate => 0.0005,
    EditField::EnergyDrainHerb | EditField::EnergyDrainCarn => 0.01,
        EditField::WorldWidth | EditField::WorldHeight => 25.0,
    EditField::InitialEnergyHerb | EditField::MaxEnergyHerb | EditField::InitialEnergyCarn | EditField::MaxEnergyCarn => 25.0,
        // fitness weights: subtle nudge
        _ => 0.05,
    }
}

fn is_integer_field(field: EditField) -> bool {
    matches!(field, EditField::PopSize | EditField::MaxFood | EditField::Herbivores | EditField::Carnivores)
}

fn field_help(field: EditField) -> &'static str {
    match field {
        EditField::WorldWidth => "Horizontal world size in units. Larger worlds spread agents.",
        EditField::WorldHeight => "Vertical world size in units.",
        EditField::PopSize => "Number of agents (population). Higher = heavier CPU load.",
        EditField::Herbivores => "Number of herbivore agents to spawn at episode start.",
        EditField::Carnivores => "Number of carnivore agents to spawn at episode start.",
        EditField::MaxFood => "Maximum number of plants present at once.",
    EditField::FoodRespawnRate => "Per-step probability a new plant appears (0.0001-0.1).",
    EditField::InitialEnergyHerb => "Starting energy for herbivores.",
    EditField::MaxEnergyHerb => "Energy cap for herbivores.",
    EditField::EnergyDrainHerb => "Metabolism drain per step for herbivores.",
    EditField::InitialEnergyCarn => "Starting energy for carnivores.",
    EditField::MaxEnergyCarn => "Energy cap for carnivores.",
    EditField::EnergyDrainCarn => "Metabolism drain per step for carnivores.",
        EditField::WLifetimeHerb | EditField::WLifetimeCarn => "Fitness weight for survival/lifespan.",
        EditField::WEnergyHerb | EditField::WEnergyCarn => "Fitness weight for ending energy.",
        EditField::WOffspringHerb | EditField::WOffspringCarn => "Fitness reward for successful reproduction.",
        EditField::WCommHerb | EditField::WCommCarn => "Weight for communication-related rewards.",
        EditField::WIdleHerb | EditField::WIdleCarn => "Penalty weight for idling (discourages stalling).",
        EditField::WPlantHerb | EditField::WPlantCarn => "Reward weight for eating plants.",
        EditField::WMeatHerb | EditField::WMeatCarn => "Reward weight for eating carcasses/prey.",
        EditField::WAttacksHerb | EditField::WAttacksCarn => "Reward/pressure for initiating attacks.",
        EditField::WKillsHerb | EditField::WKillsCarn => "Reward for lethal predation.",
        EditField::WHerdHerb | EditField::WHerdCarn => "Herding/social proximity shaping weight.",
    EditField::WApproachHerb => "Approach behavior shaping weight (Herbivore) — rewards closing on plants/carcasses while moving forward.",
        EditField::WChaseHerb => "Chasing other-species shaping weight (Herbivore).",
        EditField::WChaseSameHerb => "Chasing conspecifics shaping weight (Herbivore).",
        EditField::WApproachCarn => "Approach behavior shaping weight (Carnivore).",
        EditField::WChaseCarn => "Chasing other-species shaping weight (Carnivore).",
        EditField::WChaseSameCarn => "Chasing conspecifics shaping weight (Carnivore).",
    }
}

fn draw_tooltip(text: &str, mx: f32, my: f32, ui_scale: f32) {
    let pad = 8.0 * ui_scale;
    let font_sz = 16.0 * ui_scale;
    let dims = measure_text(text, None, font_sz as u16, 1.0);
    let w = dims.width + pad * 2.0;
    let h = font_sz + pad * 1.5;
    let x = mx + 14.0 * ui_scale;
    let y = my + 14.0 * ui_scale;
    draw_rectangle(x - 2.0 * ui_scale, y - 2.0 * ui_scale, w + 4.0 * ui_scale, h + 4.0 * ui_scale, Color::new(0.0, 0.0, 0.0, 0.35));
    draw_rectangle(x, y, w, h, Color::new(0.12, 0.12, 0.16, 0.95));
    draw_rectangle_lines(x, y, w, h, 1.0 * ui_scale, Color::new(0.45, 0.75, 1.0, 1.0));
    draw_text(text, x + pad, y + h - pad * 0.6, font_sz, WHITE);
}

// Draw a labeled editable numeric field with tooltip and +/- buttons.
// Returns via state mutations; advances yref by one row height.
fn draw_field_row(
    state: &mut MenuState,
    label: &str,
    field: EditField,
    val_display: String,
    raw_val: f32,
    fx: f32,
    vx: f32,
    yref: &mut f32,
    label_size: f32,
    value_size: f32,
    line_h: f32,
    ui_scale: f32,
) -> Option<(String, f32, f32)> {
    // Note: Visibility and clipping are handled by callers; this function assumes the row is visible.
    let (mx, my) = mouse_position();
    // Label + hover detection (tooltip drawn at end of frame)
    draw_text(label, fx, *yref, label_size, WHITE);
    let label_w = measure_text(label, None, label_size as u16, 1.0).width;
    let label_hover = mx >= fx && mx <= fx + label_w + 4.0 && my >= *yref - 20.0 && my <= *yref + 8.0;
    let tooltip = if label_hover { Some((field_help(field).to_string(), mx, my)) } else { None };

    // Value input box (scaled)
    let box_w = 190.0 * ui_scale;
    let box_h = 26.0 * ui_scale;
    let box_x = vx;
    let box_y = *yref - box_h + 6.0;
    let in_box = mx >= box_x && mx <= box_x + box_w && my >= box_y && my <= box_y + box_h;
    let editing = state.editing_field == Some(field);
    let box_fill = if editing {
        Color::new(0.20, 0.22, 0.18, 1.0)
    } else if in_box {
        Color::new(0.18, 0.25, 0.18, 1.0)
    } else {
        Color::new(0.14, 0.18, 0.14, 1.0)
    };
    let box_border = if editing {
        YELLOW
    } else if in_box {
        Color::new(0.45, 0.85, 0.55, 1.0)
    } else {
        Color::new(0.35, 0.75, 0.45, 1.0)
    };
    draw_rectangle(box_x, box_y, box_w, box_h, box_fill);
    draw_rectangle_lines(box_x, box_y, box_w, box_h, 2.0, box_border);

    // Text inside the box
    let show = if editing { format!("{}_", state.input_buffer) } else { val_display };
    draw_text(&show, box_x + 8.0, box_y + box_h - 7.0, value_size, Color::new(0.8, 1.0, 0.8, 1.0));

    // +/- nudge buttons (scaled)
    let step = field_step(field);
    let btn_w = 24.0 * ui_scale;
    let btn_h = box_h;
    let minus_x = box_x + box_w + 6.0;
    let plus_x = minus_x + btn_w + 6.0;
    let btn_y = box_y;
    let over_minus = mx >= minus_x && mx <= minus_x + btn_w && my >= btn_y && my <= btn_y + btn_h;
    let over_plus = mx >= plus_x && mx <= plus_x + btn_w && my >= btn_y && my <= btn_y + btn_h;
    let btn_col = |hover: bool| if hover { Color::new(0.25, 0.45, 0.65, 1.0) } else { Color::new(0.2, 0.35, 0.5, 1.0) };
    draw_rectangle(minus_x, btn_y, btn_w, btn_h, btn_col(over_minus));
    draw_rectangle_lines(minus_x, btn_y, btn_w, btn_h, 1.5, WHITE);
    draw_text("-", minus_x + 8.0 * ui_scale, btn_y + btn_h - 7.0 * ui_scale, value_size, WHITE);
    draw_rectangle(plus_x, btn_y, btn_w, btn_h, btn_col(over_plus));
    draw_rectangle_lines(plus_x, btn_y, btn_w, btn_h, 1.5, WHITE);
    draw_text("+", plus_x + 6.0 * ui_scale, btn_y + btn_h - 7.0 * ui_scale, value_size, WHITE);

    // Click handling: focus, +/- nudge
    if is_mouse_button_pressed(MouseButton::Left) {
        if in_box {
            state.editing_field = Some(field);
            state.input_buffer = show.trim_end_matches('_').to_string();
        } else if over_minus {
            let mut next = raw_val - step;
            if is_integer_field(field) { next = (next.round()).max(0.0); }
            apply_field_value(&mut state.config, field, next);
            state.editing_field = None;
            state.input_buffer.clear();
        } else if over_plus {
            let mut next = raw_val + step;
            if is_integer_field(field) { next = (next.round()).max(0.0); }
            apply_field_value(&mut state.config, field, next);
            state.editing_field = None;
            state.input_buffer.clear();
        }
    }

    *yref += line_h;
    tooltip
}

/// Draw the menu and handle input. Returns Some(config) when user confirms, None while still editing
pub fn draw_menu(state: &mut MenuState) -> Option<SimConfig> {
    clear_background(Color::new(0.05, 0.05, 0.08, 1.0));

    // Panel size and frozen UI scale (consistent fonts across screens)
    let w = screen_width();
    let h = screen_height();
    let panel_w = (w * 0.65).clamp(700.0, 1000.0);
    let mut panel_h = (h * 0.8).clamp(720.0, 1000.0);
    let base_panel_w = 900.0;
    let base_panel_h = 800.0;
    let ui_scale = (panel_w / base_panel_w).min(panel_h / base_panel_h).clamp(0.6, 1.6);

    let padding = 30.0 * ui_scale;
    let line_h = 35.0 * ui_scale;
    let label_size = 22.0 * ui_scale;
    let value_size = 20.0 * ui_scale;

    // Adjust panel height for Core (content-driven); keep Fitness tall for scroll area
    if state.screen == MenuScreen::Core {
        let title_and_tip_h = (line_h + 6.0) + (line_h + 10.0);
        let header_h = line_h;
        let fields_rows_left = 3.0_f32;
        let fields_rows_right = 3.0_f32;
        let fields_h = fields_rows_left.max(fields_rows_right) * line_h;
        let content_h = title_and_tip_h + header_h + fields_h + 28.0;
        let button_h = 50.0;
    let desired_h = padding + content_h + 12.0 * ui_scale + button_h + 12.0 * ui_scale;
    // Make Core panel fully content-driven (no artificial minimum), but never exceed screen
    panel_h = desired_h.min(h - 40.0);
    }

    let panel_x = (w - panel_w) / 2.0;
    let panel_y = (h - panel_h) / 2.0;
    draw_rectangle(panel_x, panel_y, panel_w, panel_h, Color::new(0.10, 0.11, 0.14, 1.0));
    draw_rectangle_lines(panel_x, panel_y, panel_w, panel_h, 3.0, Color::new(0.35, 0.55, 0.85, 1.0));

    // Header
    let mut deferred_tooltips: Vec<(String, f32, f32)> = Vec::new();
    let x = panel_x + padding;
    let mut y = panel_y + padding;
    let title = match state.screen { MenuScreen::Core => "World & Population", MenuScreen::Fitness => "Fitness & Energy Weights" };
    draw_text(title, x, y, 28.0 * ui_scale, WHITE);
    y += line_h + 6.0;
    draw_text("Hover labels for help. Click values to edit; +/- to nudge.", x, y, 18.0 * ui_scale, Color::new(0.75, 0.85, 1.0, 0.9));
    y += line_h + 10.0;

    // Top-right help button
    let help_w = 34.0 * ui_scale;
    let help_h = 28.0 * ui_scale;
    let help_x = panel_x + panel_w - padding - help_w;
    let help_y = panel_y + padding + 2.0 * ui_scale;
    let (mx, my) = mouse_position();
    let help_hover = mx >= help_x && mx <= help_x + help_w && my >= help_y && my <= help_y + help_h;
    let help_col = if help_hover { Color::new(0.30, 0.55, 0.85, 1.0) } else { Color::new(0.22, 0.45, 0.72, 1.0) };
    draw_rectangle(help_x, help_y, help_w, help_h, help_col);
    draw_rectangle_lines(help_x, help_y, help_w, help_h, 2.0 * ui_scale, WHITE);
    let q_w = measure_text("?", None, (20.0 * ui_scale) as u16, 1.0).width;
    draw_text("?", help_x + (help_w - q_w) / 2.0, help_y + help_h - 7.0 * ui_scale, 20.0 * ui_scale, WHITE);
    let mut help_opened_now = false;
    if help_hover && is_mouse_button_pressed(MouseButton::Left) { state.show_help = true; help_opened_now = true; }

    // If help is open, draw overlay and short-circuit interactions
    if state.show_help {
    draw_help_overlay(state, panel_x, panel_y, panel_w, panel_h, ui_scale);
        // Require a new click/press to close; ignore the opening click in this same frame
        if !help_opened_now && (is_mouse_button_pressed(MouseButton::Left) || is_key_pressed(KeyCode::Escape)) {
            state.show_help = false;
        }
        return None;
    }

    if state.screen == MenuScreen::Core {
        // Core screen: two columns, 3 rows each
        let inner_w = panel_w - 2.0 * padding;
        let col_w = (inner_w - 40.0 * ui_scale) * 0.5;
    let x1 = x;
    let x2 = x + col_w + 40.0 * ui_scale;
    let label_x1 = x1 + 20.0 * ui_scale;
    // Ensure the value input area fits within the column width (box + +/- buttons)
    let controls_w = (190.0 * ui_scale) + 2.0 * (24.0 * ui_scale) + 12.0; // 12.0 accounts for inner gaps
    let value_x1 = x1 + col_w - controls_w;
    let label_x2 = x2 + 20.0 * ui_scale;
    let value_x2 = x2 + col_w - controls_w;

        draw_text("Core Parameters", x, y, label_size, LIGHTGRAY);
        let core_header_y = y;
        y += line_h;
        let mut y1 = y;
        let mut y2 = y;

        // Left column
        if let Some(t) = draw_field_row(state, "World Width", EditField::WorldWidth, format!("{:.0}", state.config.world_width), state.config.world_width, label_x1, value_x1, &mut y1, label_size, value_size, line_h, ui_scale) { deferred_tooltips.push(t) }
        if let Some(t) = draw_field_row(state, "World Height", EditField::WorldHeight, format!("{:.0}", state.config.world_height), state.config.world_height, label_x1, value_x1, &mut y1, label_size, value_size, line_h, ui_scale) { deferred_tooltips.push(t) }
        if let Some(t) = draw_field_row(state, "Max Food", EditField::MaxFood, format!("{:.0}", state.config.max_food as f32), state.config.max_food as f32, label_x1, value_x1, &mut y1, label_size, value_size, line_h, ui_scale) { deferred_tooltips.push(t) }

        // Right column
        if let Some(t) = draw_field_row(state, "Population Size", EditField::PopSize, format!("{:.0}", state.config.population_size as f32), state.config.population_size as f32, label_x2, value_x2, &mut y2, label_size, value_size, line_h, ui_scale) { deferred_tooltips.push(t) }
        if let Some(t) = draw_field_row(state, "Herbivores", EditField::Herbivores, format!("{:.0}", state.config.herbivore_count as f32), state.config.herbivore_count as f32, label_x2, value_x2, &mut y2, label_size, value_size, line_h, ui_scale) { deferred_tooltips.push(t) }
        if let Some(t) = draw_field_row(state, "Carnivores", EditField::Carnivores, format!("{:.0}", state.config.carnivore_count as f32), state.config.carnivore_count as f32, label_x2, value_x2, &mut y2, label_size, value_size, line_h, ui_scale) { deferred_tooltips.push(t) }

        // Outline around the core fields
        let core_top = core_header_y - 6.0;
        let core_bottom = y1.max(y2);
        let core_h = (core_bottom - core_top) + 12.0;
        let core_panel_x = x - 14.0;
        let core_panel_w = inner_w + 28.0;
        draw_rectangle_lines(core_panel_x, core_top, core_panel_w, core_h, 1.0, Color::new(0.3, 0.6, 0.8, 0.35));

        // Buttons just below content (content-driven)
        y = core_bottom + 12.0 * ui_scale;
    } else {
        // Fitness screen: two scrollable columns with headers
        let inner_w = panel_w - 2.0 * padding;
        let sections_gap = 24.0 * ui_scale;
        let sec_w = (inner_w - sections_gap).max(0.0) * 0.5;

        let herb_x = x;
        let herb_label_x = herb_x + 20.0 * ui_scale;
        let herb_value_x = herb_label_x + 160.0 * ui_scale;
        let carn_x = x + sec_w + sections_gap;
        let carn_label_x = carn_x + 20.0 * ui_scale;
        let carn_value_x = carn_label_x + 160.0 * ui_scale;

        draw_text("Fitness Weights (Herbivore)", herb_x, y, label_size, LIGHTGRAY);
        draw_text("Fitness Weights (Carnivore)", carn_x, y, label_size, LIGHTGRAY);

        let viewport_y = y + line_h;
        let available_bottom = panel_y + panel_h - (50.0 + padding) - 8.0;
        let viewport_h = (available_bottom - viewport_y).max(40.0);

        // Scroll handling when mouse over either column area
        let (mx, my) = mouse_position();
        let over_view = my >= viewport_y && my <= viewport_y + viewport_h && mx >= x && mx <= x + inner_w;
        if over_view {
            let (_wx, wy) = mouse_wheel();
            if wy.abs() > 0.0 { state.fitness_scroll -= wy * (40.0 * ui_scale); }
        }
        let total_rows: f32 = 16.0; // per column
        let content_h = total_rows * line_h;
        let max_scroll = (content_h - viewport_h).max(0.0);
        if state.fitness_scroll < 0.0 { state.fitness_scroll = 0.0; }
        if state.fitness_scroll > max_scroll { state.fitness_scroll = max_scroll; }

        let mut yh = viewport_y - state.fitness_scroll;
        let mut yc = viewport_y - state.fitness_scroll;
    let row_visible = |row_y: f32| -> bool { row_y + line_h >= viewport_y && row_y <= viewport_y + viewport_h };
    let mut push_tt = |opt: Option<(String, f32, f32)>| { if let Some(t) = opt { deferred_tooltips.push(t); } };

        // Herbivore column rows
        let vis = row_visible(yh); push_tt(draw_field_row(state, "Init Energy (Herb)", EditField::InitialEnergyHerb, format!("{:.1}", state.config.initial_energy_herb), state.config.initial_energy_herb, herb_label_x, herb_value_x, &mut yh, label_size, value_size, line_h, ui_scale).filter(|_| vis));
        let vis = row_visible(yh); push_tt(draw_field_row(state, "Max Energy (Herb)", EditField::MaxEnergyHerb, format!("{:.1}", state.config.max_energy_herb), state.config.max_energy_herb, herb_label_x, herb_value_x, &mut yh, label_size, value_size, line_h, ui_scale).filter(|_| vis));
        let vis = row_visible(yh); push_tt(draw_field_row(state, "Drain/Step (Herb)", EditField::EnergyDrainHerb, format!("{:.3}", state.config.energy_drain_per_step_herb), state.config.energy_drain_per_step_herb, herb_label_x, herb_value_x, &mut yh, label_size, value_size, line_h, ui_scale).filter(|_| vis));
        let vis = row_visible(yh); push_tt(draw_field_row(state, "Lifetime (Herb)", EditField::WLifetimeHerb, format!("{:.3}", state.config.w_lifetime_herb), state.config.w_lifetime_herb, herb_label_x, herb_value_x, &mut yh, label_size, value_size, line_h, ui_scale).filter(|_| vis));
        let vis = row_visible(yh); push_tt(draw_field_row(state, "Energy (Herb)", EditField::WEnergyHerb, format!("{:.3}", state.config.w_energy_herb), state.config.w_energy_herb, herb_label_x, herb_value_x, &mut yh, label_size, value_size, line_h, ui_scale).filter(|_| vis));
        let vis = row_visible(yh); push_tt(draw_field_row(state, "Offspring (Herb)", EditField::WOffspringHerb, format!("{:.3}", state.config.w_offspring_herb), state.config.w_offspring_herb, herb_label_x, herb_value_x, &mut yh, label_size, value_size, line_h, ui_scale).filter(|_| vis));
        let vis = row_visible(yh); push_tt(draw_field_row(state, "Comm (Herb)", EditField::WCommHerb, format!("{:.3}", state.config.w_comm_herb), state.config.w_comm_herb, herb_label_x, herb_value_x, &mut yh, label_size, value_size, line_h, ui_scale).filter(|_| vis));
        let vis = row_visible(yh); push_tt(draw_field_row(state, "Idle Penalty (Herb)", EditField::WIdleHerb, format!("{:.3}", state.config.w_idle_penalty_herb), state.config.w_idle_penalty_herb, herb_label_x, herb_value_x, &mut yh, label_size, value_size, line_h, ui_scale).filter(|_| vis));
        let vis = row_visible(yh); push_tt(draw_field_row(state, "Plants (Herb)", EditField::WPlantHerb, format!("{:.3}", state.config.w_plant_herb), state.config.w_plant_herb, herb_label_x, herb_value_x, &mut yh, label_size, value_size, line_h, ui_scale).filter(|_| vis));
        let vis = row_visible(yh); push_tt(draw_field_row(state, "Meat (Herb)", EditField::WMeatHerb, format!("{:.3}", state.config.w_meat_herb), state.config.w_meat_herb, herb_label_x, herb_value_x, &mut yh, label_size, value_size, line_h, ui_scale).filter(|_| vis));
        let vis = row_visible(yh); push_tt(draw_field_row(state, "Attacks (Herb)", EditField::WAttacksHerb, format!("{:.3}", state.config.w_attacks_herb), state.config.w_attacks_herb, herb_label_x, herb_value_x, &mut yh, label_size, value_size, line_h, ui_scale).filter(|_| vis));
        let vis = row_visible(yh); push_tt(draw_field_row(state, "Kills (Herb)", EditField::WKillsHerb, format!("{:.3}", state.config.w_kills_herb), state.config.w_kills_herb, herb_label_x, herb_value_x, &mut yh, label_size, value_size, line_h, ui_scale).filter(|_| vis));
        let vis = row_visible(yh); push_tt(draw_field_row(state, "Herding (Herb)", EditField::WHerdHerb, format!("{:.3}", state.config.w_herding_herb), state.config.w_herding_herb, herb_label_x, herb_value_x, &mut yh, label_size, value_size, line_h, ui_scale).filter(|_| vis));
        let vis = row_visible(yh); push_tt(draw_field_row(state, "Approach (Herb)", EditField::WApproachHerb, format!("{:.3}", state.config.w_approach_herb), state.config.w_approach_herb, herb_label_x, herb_value_x, &mut yh, label_size, value_size, line_h, ui_scale).filter(|_| vis));
        let vis = row_visible(yh); push_tt(draw_field_row(state, "Chase Other (Herb)", EditField::WChaseHerb, format!("{:.3}", state.config.w_chase_herb), state.config.w_chase_herb, herb_label_x, herb_value_x, &mut yh, label_size, value_size, line_h, ui_scale).filter(|_| vis));
        let _vis_last = row_visible(yh); push_tt(draw_field_row(state, "Chase Same (Herb)", EditField::WChaseSameHerb, format!("{:.3}", state.config.w_chase_same_herb), state.config.w_chase_same_herb, herb_label_x, herb_value_x, &mut yh, label_size, value_size, line_h, ui_scale));

        // Carnivore column rows
        let vis = row_visible(yc); push_tt(draw_field_row(state, "Init Energy (Carn)", EditField::InitialEnergyCarn, format!("{:.1}", state.config.initial_energy_carn), state.config.initial_energy_carn, carn_label_x, carn_value_x, &mut yc, label_size, value_size, line_h, ui_scale).filter(|_| vis));
        let vis = row_visible(yc); push_tt(draw_field_row(state, "Max Energy (Carn)", EditField::MaxEnergyCarn, format!("{:.1}", state.config.max_energy_carn), state.config.max_energy_carn, carn_label_x, carn_value_x, &mut yc, label_size, value_size, line_h, ui_scale).filter(|_| vis));
        let vis = row_visible(yc); push_tt(draw_field_row(state, "Drain/Step (Carn)", EditField::EnergyDrainCarn, format!("{:.3}", state.config.energy_drain_per_step_carn), state.config.energy_drain_per_step_carn, carn_label_x, carn_value_x, &mut yc, label_size, value_size, line_h, ui_scale).filter(|_| vis));
        let vis = row_visible(yc); push_tt(draw_field_row(state, "Lifetime (Carn)", EditField::WLifetimeCarn, format!("{:.3}", state.config.w_lifetime_carn), state.config.w_lifetime_carn, carn_label_x, carn_value_x, &mut yc, label_size, value_size, line_h, ui_scale).filter(|_| vis));
        let vis = row_visible(yc); push_tt(draw_field_row(state, "Energy (Carn)", EditField::WEnergyCarn, format!("{:.3}", state.config.w_energy_carn), state.config.w_energy_carn, carn_label_x, carn_value_x, &mut yc, label_size, value_size, line_h, ui_scale).filter(|_| vis));
        let vis = row_visible(yc); push_tt(draw_field_row(state, "Offspring (Carn)", EditField::WOffspringCarn, format!("{:.3}", state.config.w_offspring_carn), state.config.w_offspring_carn, carn_label_x, carn_value_x, &mut yc, label_size, value_size, line_h, ui_scale).filter(|_| vis));
        let vis = row_visible(yc); push_tt(draw_field_row(state, "Comm (Carn)", EditField::WCommCarn, format!("{:.3}", state.config.w_comm_carn), state.config.w_comm_carn, carn_label_x, carn_value_x, &mut yc, label_size, value_size, line_h, ui_scale).filter(|_| vis));
        let vis = row_visible(yc); push_tt(draw_field_row(state, "Idle Penalty (Carn)", EditField::WIdleCarn, format!("{:.3}", state.config.w_idle_penalty_carn), state.config.w_idle_penalty_carn, carn_label_x, carn_value_x, &mut yc, label_size, value_size, line_h, ui_scale).filter(|_| vis));
        let vis = row_visible(yc); push_tt(draw_field_row(state, "Plants (Carn)", EditField::WPlantCarn, format!("{:.3}", state.config.w_plant_carn), state.config.w_plant_carn, carn_label_x, carn_value_x, &mut yc, label_size, value_size, line_h, ui_scale).filter(|_| vis));
        let vis = row_visible(yc); push_tt(draw_field_row(state, "Meat (Carn)", EditField::WMeatCarn, format!("{:.3}", state.config.w_meat_carn), state.config.w_meat_carn, carn_label_x, carn_value_x, &mut yc, label_size, value_size, line_h, ui_scale).filter(|_| vis));
        let vis = row_visible(yc); push_tt(draw_field_row(state, "Attacks (Carn)", EditField::WAttacksCarn, format!("{:.3}", state.config.w_attacks_carn), state.config.w_attacks_carn, carn_label_x, carn_value_x, &mut yc, label_size, value_size, line_h, ui_scale).filter(|_| vis));
        let vis = row_visible(yc); push_tt(draw_field_row(state, "Kills (Carn)", EditField::WKillsCarn, format!("{:.3}", state.config.w_kills_carn), state.config.w_kills_carn, carn_label_x, carn_value_x, &mut yc, label_size, value_size, line_h, ui_scale).filter(|_| vis));
        let vis = row_visible(yc); push_tt(draw_field_row(state, "Herding (Carn)", EditField::WHerdCarn, format!("{:.3}", state.config.w_herding_carn), state.config.w_herding_carn, carn_label_x, carn_value_x, &mut yc, label_size, value_size, line_h, ui_scale).filter(|_| vis));
        let vis = row_visible(yc); push_tt(draw_field_row(state, "Approach (Carn)", EditField::WApproachCarn, format!("{:.3}", state.config.w_approach_carn), state.config.w_approach_carn, carn_label_x, carn_value_x, &mut yc, label_size, value_size, line_h, ui_scale).filter(|_| vis));
        let vis = row_visible(yc); push_tt(draw_field_row(state, "Chase Other (Carn)", EditField::WChaseCarn, format!("{:.3}", state.config.w_chase_carn), state.config.w_chase_carn, carn_label_x, carn_value_x, &mut yc, label_size, value_size, line_h, ui_scale).filter(|_| vis));
        let _vis_last = row_visible(yc); push_tt(draw_field_row(state, "Chase Same (Carn)", EditField::WChaseSameCarn, format!("{:.3}", state.config.w_chase_same_carn), state.config.w_chase_same_carn, carn_label_x, carn_value_x, &mut yc, label_size, value_size, line_h, ui_scale));

        // View outlines
        draw_rectangle_lines(herb_x - 14.0, viewport_y - 6.0, sec_w + 28.0, viewport_h + 12.0, 1.0 * ui_scale, Color::new(0.3, 0.6, 0.8, 0.35));
        draw_rectangle_lines(carn_x - 14.0, viewport_y - 6.0, sec_w + 28.0, viewport_h + 12.0, 1.0 * ui_scale, Color::new(0.3, 0.6, 0.8, 0.35));

        // Buttons anchored to bottom
    y = panel_y + panel_h - 50.0 - padding;
    }

    // Keyboard input for editing
    if let Some(field) = state.editing_field {
        if is_key_pressed(KeyCode::Backspace) { state.input_buffer.pop(); }
        if is_key_pressed(KeyCode::Enter) {
            if let Ok(val) = state.input_buffer.parse::<f32>() { apply_field_value(&mut state.config, field, val); }
            state.editing_field = None; state.input_buffer.clear();
        }
        if is_key_pressed(KeyCode::Escape) { state.editing_field = None; state.input_buffer.clear(); }
        while let Some(ch) = get_char_pressed() { if ch.is_ascii_digit() || ch == '.' || ch == '-' { state.input_buffer.push(ch); } }
    }

    // Bottom buttons
    let button_w = 200.0;
    let button_h = 50.0;
    let button_gap = 20.0;
    let total_buttons = if state.screen == MenuScreen::Core { 2 } else { 3 };
    let buttons_total_w = button_w * (total_buttons as f32) + button_gap * ((total_buttons - 1) as f32);
    let start_x_base = panel_x + (panel_w - buttons_total_w) / 2.0;
    let reset_x = start_x_base;
    let back_x = if state.screen == MenuScreen::Fitness { reset_x + button_w + button_gap } else { 0.0 };
    let next_x = if state.screen == MenuScreen::Core { reset_x + button_w + button_gap } else { 0.0 };
    let start_x = if state.screen == MenuScreen::Fitness { back_x + button_w + button_gap } else { 0.0 };

    let (mx, my) = mouse_position();
    let btn_color = |hover: bool| if hover { Color::new(0.35, 0.75, 0.95, 1.0) } else { Color::new(0.22, 0.55, 0.78, 1.0) };

    // Reset button
    let hover_reset = mx >= reset_x && mx <= reset_x + button_w && my >= y && my <= y + button_h;
    draw_rectangle(reset_x, y, button_w, button_h, btn_color(hover_reset));
    draw_rectangle_lines(reset_x, y, button_w, button_h, 2.0 * ui_scale, WHITE);
    let reset_label = "RESET TO DEFAULTS";
    let rtw = measure_text(reset_label, None, 20, 1.0).width;
    draw_text(reset_label, reset_x + (button_w - rtw) / 2.0, y + 32.0 * ui_scale, 20.0 * ui_scale, WHITE);
    if hover_reset && is_mouse_button_pressed(MouseButton::Left) && state.editing_field.is_none() { *state = MenuState::new(); }

    if state.screen == MenuScreen::Core {
        // Next button
        let hover_next = mx >= next_x && mx <= next_x + button_w && my >= y && my <= y + button_h;
        draw_rectangle(next_x, y, button_w, button_h, btn_color(hover_next));
        draw_rectangle_lines(next_x, y, button_w, button_h, 2.0 * ui_scale, WHITE);
        let label = "NEXT: FITNESS";
        let tw = measure_text(label, None, 22, 1.0).width;
        draw_text(label, next_x + (button_w - tw) / 2.0, y + 32.0 * ui_scale, 22.0 * ui_scale, WHITE);
        if hover_next && is_mouse_button_pressed(MouseButton::Left) && state.editing_field.is_none() { state.screen = MenuScreen::Fitness; }
    } else {
        // Back button
        let hover_back = mx >= back_x && mx <= back_x + button_w && my >= y && my <= y + button_h;
        draw_rectangle(back_x, y, button_w, button_h, btn_color(hover_back));
        draw_rectangle_lines(back_x, y, button_w, button_h, 2.0 * ui_scale, WHITE);
        let label = "BACK";
        let tw = measure_text(label, None, 22, 1.0).width;
        draw_text(label, back_x + (button_w - tw) / 2.0, y + 32.0 * ui_scale, 22.0 * ui_scale, WHITE);
        if hover_back && is_mouse_button_pressed(MouseButton::Left) && state.editing_field.is_none() { state.screen = MenuScreen::Core; }

        // Start button
        let hover_start = mx >= start_x && mx <= start_x + button_w && my >= y && my <= y + button_h;
        draw_rectangle(start_x, y, button_w, button_h, btn_color(hover_start));
        draw_rectangle_lines(start_x, y, button_w, button_h, 2.0 * ui_scale, WHITE);
        let text = "START SIMULATION";
        let text_w = measure_text(text, None, 24, 1.0).width;
        draw_text(text, start_x + (button_w - text_w) / 2.0, y + 32.0 * ui_scale, 24.0 * ui_scale, WHITE);
        if hover_start && is_mouse_button_pressed(MouseButton::Left) && state.editing_field.is_none() { return Some(state.config.clone()); }
    }

    // Draw last tooltip on top
    if let Some((text, tx, ty)) = deferred_tooltips.last() { draw_tooltip(text, *tx, *ty, ui_scale); }

    None
}
