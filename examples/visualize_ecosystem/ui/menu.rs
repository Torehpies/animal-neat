//! Simple menu for customizing simulation parameters before starting

use macroquad::prelude::*;

// Pull in backend types/logic and re-export so external modules can continue using ui_menu::SimConfig
pub use crate::menu_backend::{SimConfig, MenuState};
use crate::menu_backend::{EditField, apply_field_value};

// --- Small UI helpers for clearer, discoverable editing ---
fn field_step(field: EditField) -> f32 {
    match field {
        EditField::PopSize | EditField::MaxFood => 10.0,
        EditField::FoodRespawnRate => 0.0005,
        EditField::EnergyDrain => 0.01,
        EditField::WorldWidth | EditField::WorldHeight => 25.0,
        EditField::InitialEnergy | EditField::MaxEnergy => 25.0,
        // fitness weights: subtle nudge
        _ => 0.05,
    }
}

fn is_integer_field(field: EditField) -> bool {
    matches!(field, EditField::PopSize | EditField::MaxFood)
}

fn field_help(field: EditField) -> &'static str {
    match field {
        EditField::WorldWidth => "Horizontal world size in units. Larger worlds spread agents.",
        EditField::WorldHeight => "Vertical world size in units.",
        EditField::PopSize => "Number of agents (population). Higher = heavier CPU load.",
        EditField::MaxFood => "Maximum number of plants present at once.",
        EditField::FoodRespawnRate => "Per-step probability a new plant appears (0.0001–0.1).",
        EditField::InitialEnergy => "Starting energy for each agent.",
        EditField::MaxEnergy => "Energy cap; agents cannot exceed this.",
        EditField::EnergyDrain => "Energy consumed by metabolism each step.",
        EditField::WLifetime => "Fitness weight for survival/lifespan.",
        EditField::WEnergy => "Fitness weight for ending energy.",
        EditField::WOffspring => "Fitness reward for successful reproduction.",
        EditField::WComm => "Weight for communication-related rewards.",
        EditField::WIdle => "Penalty weight for idling (discourages stalling).",
        EditField::WPlant => "Reward weight for eating plants.",
        EditField::WMeat => "Reward weight for eating carcasses/prey.",
        EditField::WAttacks => "Reward/pressure for initiating attacks.",
        EditField::WKills => "Reward for lethal predation.",
        EditField::WHerd => "Herding/social proximity shaping weight.",
        EditField::WApproach => "Approach behavior shaping weight.",
        EditField::WChase => "Chasing behavior shaping weight.",
        EditField::WChaseSame => "Chasing conspecifics shaping weight.",
    }
}

fn draw_tooltip(text: &str, mx: f32, my: f32) {
    let pad = 8.0;
    let font_sz = 16.0;
    let dims = measure_text(text, None, font_sz as u16, 1.0);
    let w = dims.width + pad * 2.0;
    let h = font_sz + pad * 1.5;
    let x = mx + 14.0;
    let y = my + 14.0;
    draw_rectangle(x - 2.0, y - 2.0, w + 4.0, h + 4.0, Color::new(0.0, 0.0, 0.0, 0.35));
    draw_rectangle(x, y, w, h, Color::new(0.12, 0.12, 0.16, 0.95));
    draw_rectangle_lines(x, y, w, h, 1.0, Color::new(0.45, 0.75, 1.0, 1.0));
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
) -> Option<(String, f32, f32)> {
    let (mx, my) = mouse_position();
    // Label + hover detection (tooltip drawn at end of frame)
    draw_text(label, fx, *yref, label_size, WHITE);
    let label_w = measure_text(label, None, label_size as u16, 1.0).width;
    let label_hover = mx >= fx && mx <= fx + label_w + 4.0 && my >= *yref - 20.0 && my <= *yref + 8.0;
    let tooltip = if label_hover { Some((field_help(field).to_string(), mx, my)) } else { None };

    // Value input box
    let box_w = 190.0;
    let box_h = 26.0;
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

    // +/- nudge buttons
    let step = field_step(field);
    let btn_w = 24.0;
    let btn_h = box_h;
    let minus_x = box_x + box_w + 6.0;
    let plus_x = minus_x + btn_w + 6.0;
    let btn_y = box_y;
    let over_minus = mx >= minus_x && mx <= minus_x + btn_w && my >= btn_y && my <= btn_y + btn_h;
    let over_plus = mx >= plus_x && mx <= plus_x + btn_w && my >= btn_y && my <= btn_y + btn_h;
    let btn_col = |hover: bool| if hover { Color::new(0.25, 0.45, 0.65, 1.0) } else { Color::new(0.2, 0.35, 0.5, 1.0) };
    draw_rectangle(minus_x, btn_y, btn_w, btn_h, btn_col(over_minus));
    draw_rectangle_lines(minus_x, btn_y, btn_w, btn_h, 1.5, WHITE);
    draw_text("-", minus_x + 8.0, btn_y + btn_h - 7.0, value_size, WHITE);
    draw_rectangle(plus_x, btn_y, btn_w, btn_h, btn_col(over_plus));
    draw_rectangle_lines(plus_x, btn_y, btn_w, btn_h, 1.5, WHITE);
    draw_text("+", plus_x + 6.0, btn_y + btn_h - 7.0, value_size, WHITE);

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
    
    let w = screen_width();
    let h = screen_height();
    let panel_w = (w * 0.65).clamp(700.0, 1000.0);
    let panel_h = (h * 0.8).clamp(720.0, 1000.0);
    let panel_x = (w - panel_w) / 2.0;
    let panel_y = (h - panel_h) / 2.0;
    
    // Panel background
    draw_rectangle(panel_x, panel_y, panel_w, panel_h, Color::new(0.12, 0.12, 0.15, 1.0));
    draw_rectangle_lines(panel_x, panel_y, panel_w, panel_h, 3.0, Color::new(0.3, 0.6, 0.8, 1.0));
    
    let padding = 30.0;
    let mut y = panel_y + padding;
    let x = panel_x + padding;
    let line_h = 35.0;
    let title_size = 32.0;
    let label_size = 18.0;
    let value_size = 18.0;
    
    // Title + quick instructions
    draw_text("Simulation Configuration", x, y, title_size, WHITE);
    y += line_h + 6.0;
    draw_text(
        "Tip: Click inside the green boxes to edit; use +/- to nudge. Hover labels for help.",
        x,
        y,
        16.0,
        Color::new(0.75, 0.75, 0.8, 1.0),
    );
    y += line_h + 10.0;
    
    // Core parameters header with a subtle subpanel outline
    let core_header_y = y;
    draw_text("Core Parameters", x, core_header_y, label_size, LIGHTGRAY);
    y = core_header_y + line_h;
    
    // Two-column layout for core parameters
    let inner_w = panel_w - 2.0 * padding;
    let field_x1 = x + 20.0;
    let value_x1 = field_x1 + 180.0;
    let field_x2 = x + inner_w * 0.52;
    let value_x2 = field_x2 + 180.0;

    let mut y1 = y;
    let mut y2 = y;

    // (Helper moved to draw_field_row function above)

    // Collect tooltips to render on top at end
    let mut deferred_tooltips: Vec<(String, f32, f32)> = Vec::new();

    // Left column
    if let Some(t) = draw_field_row(state, "World Width", EditField::WorldWidth, format!("{:.0}", state.config.world_width), state.config.world_width, field_x1, value_x1, &mut y1, label_size, value_size, line_h) { deferred_tooltips.push(t); }
    if let Some(t) = draw_field_row(state, "World Height", EditField::WorldHeight, format!("{:.0}", state.config.world_height), state.config.world_height, field_x1, value_x1, &mut y1, label_size, value_size, line_h) { deferred_tooltips.push(t); }
    if let Some(t) = draw_field_row(state, "Agents", EditField::PopSize, format!("{}", state.config.population_size), state.config.population_size as f32, field_x1, value_x1, &mut y1, label_size, value_size, line_h) { deferred_tooltips.push(t); }
    if let Some(t) = draw_field_row(state, "Initial Energy", EditField::InitialEnergy, format!("{:.1}", state.config.initial_energy), state.config.initial_energy, field_x1, value_x1, &mut y1, label_size, value_size, line_h) { deferred_tooltips.push(t); }
    if let Some(t) = draw_field_row(state, "Max Energy", EditField::MaxEnergy, format!("{:.1}", state.config.max_energy), state.config.max_energy, field_x1, value_x1, &mut y1, label_size, value_size, line_h) { deferred_tooltips.push(t); }
    if let Some(t) = draw_field_row(state, "Drain/Step", EditField::EnergyDrain, format!("{:.3}", state.config.energy_drain_per_step), state.config.energy_drain_per_step, field_x1, value_x1, &mut y1, label_size, value_size, line_h) { deferred_tooltips.push(t); }

    // Right column
    if let Some(t) = draw_field_row(state, "Max Food", EditField::MaxFood, format!("{}", state.config.max_food), state.config.max_food as f32, field_x2, value_x2, &mut y2, label_size, value_size, line_h) { deferred_tooltips.push(t); }
    if let Some(t) = draw_field_row(state, "Spawn Rate", EditField::FoodRespawnRate, format!("{:.4}", state.config.food_respawn_prob), state.config.food_respawn_prob, field_x2, value_x2, &mut y2, label_size, value_size, line_h) { deferred_tooltips.push(t); }

    // Core subpanel outline bounds (from header to last field)
    let core_top = core_header_y - 6.0;
    let core_bottom = y1.max(y2);
    let core_h = (core_bottom - core_top) + 12.0;
    let core_panel_x = x - 14.0;
    let core_panel_w = (panel_w - 2.0 * padding) + 28.0;
    draw_rectangle_lines(core_panel_x, core_top, core_panel_w, core_h, 1.0, Color::new(0.3, 0.6, 0.8, 0.35));

    y = y1.max(y2) + 28.0;

    // Fitness Weights section (two columns)
    let fit_header_y = y;
    draw_text("Fitness Weights", x, fit_header_y, label_size, LIGHTGRAY);
    y = fit_header_y + line_h;

    let mut wyl = y;
    let mut wyr = y;
    // Left column weights
    if let Some(t) = draw_field_row(state, "Lifetime", EditField::WLifetime, format!("{:.3}", state.config.w_lifetime), state.config.w_lifetime, field_x1, field_x1 + 170.0, &mut wyl, label_size, value_size, line_h) { deferred_tooltips.push(t); }
    if let Some(t) = draw_field_row(state, "Energy", EditField::WEnergy, format!("{:.3}", state.config.w_energy), state.config.w_energy, field_x1, field_x1 + 170.0, &mut wyl, label_size, value_size, line_h) { deferred_tooltips.push(t); }
    if let Some(t) = draw_field_row(state, "Offspring", EditField::WOffspring, format!("{:.3}", state.config.w_offspring), state.config.w_offspring, field_x1, field_x1 + 170.0, &mut wyl, label_size, value_size, line_h) { deferred_tooltips.push(t); }
    if let Some(t) = draw_field_row(state, "Comm", EditField::WComm, format!("{:.3}", state.config.w_comm), state.config.w_comm, field_x1, field_x1 + 170.0, &mut wyl, label_size, value_size, line_h) { deferred_tooltips.push(t); }
    if let Some(t) = draw_field_row(state, "Idle Penalty", EditField::WIdle, format!("{:.3}", state.config.w_idle_penalty), state.config.w_idle_penalty, field_x1, field_x1 + 170.0, &mut wyl, label_size, value_size, line_h) { deferred_tooltips.push(t); }
    if let Some(t) = draw_field_row(state, "Plants", EditField::WPlant, format!("{:.3}", state.config.w_plant), state.config.w_plant, field_x1, field_x1 + 170.0, &mut wyl, label_size, value_size, line_h) { deferred_tooltips.push(t); }

    // Right column weights
    if let Some(t) = draw_field_row(state, "Meat", EditField::WMeat, format!("{:.3}", state.config.w_meat), state.config.w_meat, field_x2, field_x2 + 170.0, &mut wyr, label_size, value_size, line_h) { deferred_tooltips.push(t); }
    if let Some(t) = draw_field_row(state, "Attacks", EditField::WAttacks, format!("{:.3}", state.config.w_attacks), state.config.w_attacks, field_x2, field_x2 + 170.0, &mut wyr, label_size, value_size, line_h) { deferred_tooltips.push(t); }
    if let Some(t) = draw_field_row(state, "Kills", EditField::WKills, format!("{:.3}", state.config.w_kills), state.config.w_kills, field_x2, field_x2 + 170.0, &mut wyr, label_size, value_size, line_h) { deferred_tooltips.push(t); }
    if let Some(t) = draw_field_row(state, "Herding", EditField::WHerd, format!("{:.3}", state.config.w_herding), state.config.w_herding, field_x2, field_x2 + 170.0, &mut wyr, label_size, value_size, line_h) { deferred_tooltips.push(t); }
    if let Some(t) = draw_field_row(state, "Approach", EditField::WApproach, format!("{:.3}", state.config.w_approach), state.config.w_approach, field_x2, field_x2 + 170.0, &mut wyr, label_size, value_size, line_h) { deferred_tooltips.push(t); }
    if let Some(t) = draw_field_row(state, "Chase", EditField::WChase, format!("{:.3}", state.config.w_chase), state.config.w_chase, field_x2, field_x2 + 170.0, &mut wyr, label_size, value_size, line_h) { deferred_tooltips.push(t); }
    if let Some(t) = draw_field_row(state, "Chase Same", EditField::WChaseSame, format!("{:.3}", state.config.w_chase_same), state.config.w_chase_same, field_x2, field_x2 + 170.0, &mut wyr, label_size, value_size, line_h) { deferred_tooltips.push(t); }

    // Fitness subpanel outline
    let fit_top = fit_header_y - 6.0;
    let fit_bottom = wyl.max(wyr);
    // Prevent the fitness outline from overlapping the bottom buttons. We compute
    // an available bottom using the same `padding` and the button height used
    // later (50.0). If the computed fit_bottom would extend below that we
    // clamp it so the outline stops above the buttons.
    let button_h_for_clamp = 40.0; // matches the button_h value below
    let available_bottom = panel_y + panel_h - (button_h_for_clamp + padding);
    // keep a small margin above the buttons
    let available_bottom = available_bottom - 8.0;
    let fit_bottom = fit_bottom.min(available_bottom);
    // ensure a sensible minimum height so the outline doesn't collapse
    let fit_h = ((fit_bottom - fit_top) + 12.0).max(40.0);
    let fit_panel_x = x - 14.0;
    let fit_panel_w = (panel_w - 2.0 * padding) + 28.0;
    draw_rectangle_lines(fit_panel_x, fit_top, fit_panel_w, fit_h, 1.0, Color::new(0.3, 0.6, 0.8, 0.35));

    // Handle keyboard input for editing
    if let Some(field) = state.editing_field {
        if is_key_pressed(KeyCode::Backspace) {
            state.input_buffer.pop();
        }
        
        if is_key_pressed(KeyCode::Enter) {
            // Apply the edited value
            if let Ok(val) = state.input_buffer.parse::<f32>() {
                apply_field_value(&mut state.config, field, val);
            }
            state.editing_field = None;
            state.input_buffer.clear();
        }
        
        if is_key_pressed(KeyCode::Escape) {
            state.editing_field = None;
            state.input_buffer.clear();
        }
        
        // Collect typed characters using macroquad's get_char_pressed
        while let Some(ch) = get_char_pressed() {
            if ch.is_ascii_digit() || ch == '.' || ch == '-' {
                state.input_buffer.push(ch);
            }
        }
    }
    
    // Start + Reset buttons row
    let button_w = 200.0;
    let button_h = 50.0;
    let button_y_offset = 20.0;
    let button_gap = 20.0;
    let button_x = panel_x + (panel_w - (button_w * 2.0 + button_gap)) / 2.0;
    let button_y = panel_y + panel_h - button_h - padding + button_y_offset;
    let reset_x = button_x;
    let start_x = reset_x + button_w + button_gap;
    
    // Instructions (draw after calculating button position to avoid overlap)
    draw_text(
        "Enter to confirm edit; Esc to cancel.",
        x,
        button_y - 18.0,
        16.0,
        Color::new(0.7, 0.7, 0.7, 1.0),
    );
    
    let (mx, my) = mouse_position();
    let hover_reset = mx >= reset_x && mx <= reset_x + button_w && my >= button_y && my <= button_y + button_h;
    let hover_start = mx >= start_x && mx <= start_x + button_w && my >= button_y && my <= button_y + button_h;

    let btn_color = |hover: bool| if hover { Color::new(0.35, 0.75, 0.95, 1.0) } else { Color::new(0.22, 0.55, 0.78, 1.0) };

    // Reset button
    draw_rectangle(reset_x, button_y, button_w, button_h, btn_color(hover_reset));
    draw_rectangle_lines(reset_x, button_y, button_w, button_h, 2.0, WHITE);
    let reset_label = "RESET TO DEFAULTS";
    let rtw = measure_text(reset_label, None, 20, 1.0).width;
    draw_text(reset_label, reset_x + (button_w - rtw) / 2.0, button_y + 32.0, 20.0, WHITE);
    if hover_reset && is_mouse_button_pressed(MouseButton::Left) && state.editing_field.is_none() {
        *state = MenuState::new();
    }

    // Start button
    draw_rectangle(start_x, button_y, button_w, button_h, btn_color(hover_start));
    draw_rectangle_lines(start_x, button_y, button_w, button_h, 2.0, WHITE);
    let text = "START SIMULATION";
    let text_w = measure_text(text, None, 24, 1.0).width;
    draw_text(text, start_x + (button_w - text_w) / 2.0, button_y + 32.0, 24.0, WHITE);
    if hover_start && is_mouse_button_pressed(MouseButton::Left) && state.editing_field.is_none() {
        return Some(state.config.clone());
    }
    // Draw the last tooltip (top-most hovered label) last
    if let Some((text, tx, ty)) = deferred_tooltips.last() {
        draw_tooltip(text, *tx, *ty);
    }
    
    None
}
