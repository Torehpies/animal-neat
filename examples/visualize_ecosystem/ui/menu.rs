//! Simple menu for customizing simulation parameters before starting

use macroquad::prelude::*;

#[derive(Clone, Debug)]
pub struct SimConfig {
    pub world_width: f32,
    pub world_height: f32,
    pub population_size: usize,
    pub max_food: usize,
    pub food_respawn_prob: f32,
    pub initial_energy: f32,
    pub max_energy: f32,
    pub energy_drain_per_step: f32,
}

impl Default for SimConfig {
    fn default() -> Self {
        Self {
            world_width: 750.0,
            world_height: 750.0,
            population_size: 50,
            max_food: 300,
            food_respawn_prob: 0.006,
            initial_energy: 500.0,
            max_energy: 5000.0,
            energy_drain_per_step: 0.05,
        }
    }
}

pub struct MenuState {
    pub config: SimConfig,
    editing_field: Option<EditField>,
    input_buffer: String,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum EditField {
    WorldWidth,
    WorldHeight,
    PopSize,
    MaxFood,
    FoodRespawnRate,
    InitialEnergy,
    MaxEnergy,
    EnergyDrain,
}

impl MenuState {
    pub fn new() -> Self {
        Self {
            config: SimConfig::default(),
            editing_field: None,
            input_buffer: String::new(),
        }
    }
}

/// Draw the menu and handle input. Returns Some(config) when user confirms, None while still editing
pub fn draw_menu(state: &mut MenuState) -> Option<SimConfig> {
    clear_background(Color::new(0.05, 0.05, 0.08, 1.0));
    
    let w = screen_width();
    let h = screen_height();
    let panel_w = 600.0;
    let panel_h = 620.0;
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
    let label_size = 20.0;
    let value_size = 20.0;
    
    // Title
    draw_text("Simulation Configuration", x, y, title_size, WHITE);
    y += line_h + 20.0;
    
    // World dimensions
    draw_text("World Size:", x, y, label_size, LIGHTGRAY);
    y += line_h;
    
    let field_x = x + 20.0;
    let value_x = field_x + 200.0;
    
    // World Width
    draw_text("Width:", field_x, y, label_size, WHITE);
    let is_editing_width = state.editing_field == Some(EditField::WorldWidth);
    let width_text = if is_editing_width {
        format!("{}_", state.input_buffer)
    } else {
        format!("{:.0}", state.config.world_width)
    };
    let width_color = if is_editing_width { YELLOW } else { Color::new(0.5, 0.9, 0.5, 1.0) };
    draw_text(&width_text, value_x, y, value_size, width_color);
    if is_mouse_button_pressed(MouseButton::Left) {
        let mx = mouse_position().0;
        let my = mouse_position().1;
        if mx >= value_x && mx <= value_x + 150.0 && my >= y - 20.0 && my <= y + 5.0 {
            state.editing_field = Some(EditField::WorldWidth);
            state.input_buffer = format!("{:.0}", state.config.world_width);
        }
    }
    y += line_h;
    
    // World Height
    draw_text("Height:", field_x, y, label_size, WHITE);
    let is_editing_height = state.editing_field == Some(EditField::WorldHeight);
    let height_text = if is_editing_height {
        format!("{}_", state.input_buffer)
    } else {
        format!("{:.0}", state.config.world_height)
    };
    let height_color = if is_editing_height { YELLOW } else { Color::new(0.5, 0.9, 0.5, 1.0) };
    draw_text(&height_text, value_x, y, value_size, height_color);
    if is_mouse_button_pressed(MouseButton::Left) {
        let mx = mouse_position().0;
        let my = mouse_position().1;
        if mx >= value_x && mx <= value_x + 150.0 && my >= y - 20.0 && my <= y + 5.0 {
            state.editing_field = Some(EditField::WorldHeight);
            state.input_buffer = format!("{:.0}", state.config.world_height);
        }
    }
    y += line_h + 15.0;
    
    // Population
    draw_text("Population:", x, y, label_size, LIGHTGRAY);
    y += line_h;
    
    draw_text("Agents:", field_x, y, label_size, WHITE);
    let is_editing_pop = state.editing_field == Some(EditField::PopSize);
    let pop_text = if is_editing_pop {
        format!("{}_", state.input_buffer)
    } else {
        format!("{}", state.config.population_size)
    };
    let pop_color = if is_editing_pop { YELLOW } else { Color::new(0.5, 0.9, 0.5, 1.0) };
    draw_text(&pop_text, value_x, y, value_size, pop_color);
    if is_mouse_button_pressed(MouseButton::Left) {
        let mx = mouse_position().0;
        let my = mouse_position().1;
        if mx >= value_x && mx <= value_x + 150.0 && my >= y - 20.0 && my <= y + 5.0 {
            state.editing_field = Some(EditField::PopSize);
            state.input_buffer = format!("{}", state.config.population_size);
        }
    }
    y += line_h + 15.0;
    
    // Food settings
    draw_text("Vegetation:", x, y, label_size, LIGHTGRAY);
    y += line_h;
    
    draw_text("Max Count:", field_x, y, label_size, WHITE);
    let is_editing_food = state.editing_field == Some(EditField::MaxFood);
    let food_text = if is_editing_food {
        format!("{}_", state.input_buffer)
    } else {
        format!("{}", state.config.max_food)
    };
    let food_color = if is_editing_food { YELLOW } else { Color::new(0.5, 0.9, 0.5, 1.0) };
    draw_text(&food_text, value_x, y, value_size, food_color);
    if is_mouse_button_pressed(MouseButton::Left) {
        let mx = mouse_position().0;
        let my = mouse_position().1;
        if mx >= value_x && mx <= value_x + 150.0 && my >= y - 20.0 && my <= y + 5.0 {
            state.editing_field = Some(EditField::MaxFood);
            state.input_buffer = format!("{}", state.config.max_food);
        }
    }
    y += line_h;
    
    draw_text("Spawn Rate:", field_x, y, label_size, WHITE);
    let is_editing_rate = state.editing_field == Some(EditField::FoodRespawnRate);
    let rate_text = if is_editing_rate {
        format!("{}_", state.input_buffer)
    } else {
        format!("{:.4}", state.config.food_respawn_prob)
    };
    let rate_color = if is_editing_rate { YELLOW } else { Color::new(0.5, 0.9, 0.5, 1.0) };
    draw_text(&rate_text, value_x, y, value_size, rate_color);
    if is_mouse_button_pressed(MouseButton::Left) {
        let mx = mouse_position().0;
        let my = mouse_position().1;
        if mx >= value_x && mx <= value_x + 150.0 && my >= y - 20.0 && my <= y + 5.0 {
            state.editing_field = Some(EditField::FoodRespawnRate);
            state.input_buffer = format!("{:.4}", state.config.food_respawn_prob);
        }
    }
    y += line_h + 15.0;
    
    // Energy settings
    draw_text("Energy:", x, y, label_size, LIGHTGRAY);
    y += line_h;
    
    draw_text("Initial:", field_x, y, label_size, WHITE);
    let is_editing_init_energy = state.editing_field == Some(EditField::InitialEnergy);
    let init_energy_text = if is_editing_init_energy {
        format!("{}_", state.input_buffer)
    } else {
        format!("{:.1}", state.config.initial_energy)
    };
    let init_energy_color = if is_editing_init_energy { YELLOW } else { Color::new(0.5, 0.9, 0.5, 1.0) };
    draw_text(&init_energy_text, value_x, y, value_size, init_energy_color);
    if is_mouse_button_pressed(MouseButton::Left) {
        let mx = mouse_position().0;
        let my = mouse_position().1;
        if mx >= value_x && mx <= value_x + 150.0 && my >= y - 20.0 && my <= y + 5.0 {
            state.editing_field = Some(EditField::InitialEnergy);
            state.input_buffer = format!("{:.1}", state.config.initial_energy);
        }
    }
    y += line_h;
    
    draw_text("Maximum:", field_x, y, label_size, WHITE);
    let is_editing_max_energy = state.editing_field == Some(EditField::MaxEnergy);
    let max_energy_text = if is_editing_max_energy {
        format!("{}_", state.input_buffer)
    } else {
        format!("{:.1}", state.config.max_energy)
    };
    let max_energy_color = if is_editing_max_energy { YELLOW } else { Color::new(0.5, 0.9, 0.5, 1.0) };
    draw_text(&max_energy_text, value_x, y, value_size, max_energy_color);
    if is_mouse_button_pressed(MouseButton::Left) {
        let mx = mouse_position().0;
        let my = mouse_position().1;
        if mx >= value_x && mx <= value_x + 150.0 && my >= y - 20.0 && my <= y + 5.0 {
            state.editing_field = Some(EditField::MaxEnergy);
            state.input_buffer = format!("{:.1}", state.config.max_energy);
        }
    }
    y += line_h;
    
    draw_text("Drain/Step:", field_x, y, label_size, WHITE);
    let is_editing_drain = state.editing_field == Some(EditField::EnergyDrain);
    let drain_text = if is_editing_drain {
        format!("{}_", state.input_buffer)
    } else {
        format!("{:.3}", state.config.energy_drain_per_step)
    };
    let drain_color = if is_editing_drain { YELLOW } else { Color::new(0.5, 0.9, 0.5, 1.0) };
    draw_text(&drain_text, value_x, y, value_size, drain_color);
    if is_mouse_button_pressed(MouseButton::Left) {
        let mx = mouse_position().0;
        let my = mouse_position().1;
        if mx >= value_x && mx <= value_x + 150.0 && my >= y - 20.0 && my <= y + 5.0 {
            state.editing_field = Some(EditField::EnergyDrain);
            state.input_buffer = format!("{:.3}", state.config.energy_drain_per_step);
        }
    }
    // spacing before keyboard input help; no need to update y further here
    
    // Handle keyboard input for editing
    if let Some(field) = state.editing_field {
        // Handle special keys
        if is_key_pressed(KeyCode::Backspace) {
            state.input_buffer.pop();
        }
        
        if is_key_pressed(KeyCode::Enter) {
            // Apply the edited value
            if let Ok(val) = state.input_buffer.parse::<f32>() {
                match field {
                    EditField::WorldWidth => {
                        state.config.world_width = val.max(100.0).min(2000.0);
                    }
                    EditField::WorldHeight => {
                        state.config.world_height = val.max(100.0).min(2000.0);
                    }
                    EditField::PopSize => {
                        // Allow much larger populations for stress-testing; clamp to 999,999
                        state.config.population_size = (val as usize).max(1).min(999_999);
                    }
                    EditField::MaxFood => {
                        // Allow a very large vegetation count; clamp to 999,999
                        state.config.max_food = (val as usize).max(10).min(999_999);
                    }
                    EditField::FoodRespawnRate => {
                        state.config.food_respawn_prob = val.max(0.0001).min(0.1);
                    }
                    EditField::InitialEnergy => {
                        state.config.initial_energy = val.max(10.0).min(10000.0);
                    }
                    EditField::MaxEnergy => {
                        state.config.max_energy = val.max(100.0).min(50000.0);
                    }
                    EditField::EnergyDrain => {
                        state.config.energy_drain_per_step = val.max(0.0).min(10.0);
                    }
                }
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
            if ch.is_ascii_digit() || ch == '.' {
                state.input_buffer.push(ch);
            }
        }
    }
    
    // Start button
    let button_w = 200.0;
    let button_h = 50.0;
    let button_x = panel_x + (panel_w - button_w) / 2.0;
    let button_y = panel_y + panel_h - button_h - padding;
    
    // Instructions (draw after calculating button position to avoid overlap)
    draw_text("Click on values to edit. Press Enter to confirm, Esc to cancel.", 
              x, button_y - 10.0, 16.0, Color::new(0.7, 0.7, 0.7, 1.0));
    
    let mx = mouse_position().0;
    let my = mouse_position().1;
    let hovering = mx >= button_x && mx <= button_x + button_w 
                && my >= button_y && my <= button_y + button_h;
    
    let button_color = if hovering {
        Color::new(0.3, 0.7, 0.9, 1.0)
    } else {
        Color::new(0.2, 0.5, 0.7, 1.0)
    };
    
    draw_rectangle(button_x, button_y, button_w, button_h, button_color);
    draw_rectangle_lines(button_x, button_y, button_w, button_h, 2.0, WHITE);
    
    let text = "START SIMULATION";
    let text_w = measure_text(text, None, 24, 1.0).width;
    draw_text(text, button_x + (button_w - text_w) / 2.0, button_y + 32.0, 24.0, WHITE);
    
    // Check if start button clicked
    if hovering && is_mouse_button_pressed(MouseButton::Left) && state.editing_field.is_none() {
        return Some(state.config.clone());
    }
    
    None
}
