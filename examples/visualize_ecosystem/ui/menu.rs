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
    // Fitness weights
    pub w_lifetime: f32,
    pub w_energy: f32,
    pub w_offspring: f32,
    pub w_comm: f32,
    pub w_idle_penalty: f32,
    pub w_plant: f32,
    pub w_meat: f32,
    pub w_attacks: f32,
    pub w_kills: f32,
    pub w_herding: f32,
    pub w_approach: f32,
    pub w_chase: f32,
    pub w_chase_same: f32,
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
            w_lifetime: 0.05,
            w_energy: 5.0,
            w_offspring: 10.0,
            w_comm: 0.0,
            w_idle_penalty: 0.6,
            w_plant: 2.0,
            w_meat: 20.0,
            w_attacks: 10.0,
            w_kills: 20.0,
            w_herding: 0.5,
            w_approach: 0.1,
            w_chase: 0.3,
            w_chase_same: 0.4,
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
    WLifetime,
    WEnergy,
    WOffspring,
    WComm,
    WIdle,
    WPlant,
    WMeat,
    WAttacks,
    WKills,
    WHerd,
    WApproach,
    WChase,
    WChaseSame,
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
    
    // Title
    draw_text("Simulation Configuration", x, y, title_size, WHITE);
    y += line_h + 20.0;
    
    // Core parameters header
    draw_text("Core Parameters:", x, y, label_size, LIGHTGRAY);
    y += line_h;
    
    // Two-column layout for core parameters
    let inner_w = panel_w - 2.0 * padding;
    let field_x1 = x + 20.0;
    let value_x1 = field_x1 + 180.0;
    let field_x2 = x + inner_w * 0.52;
    let value_x2 = field_x2 + 180.0;

    let mut y1 = y;
    let mut y2 = y;

    // Helper to draw a numeric field with editing support
    let mut draw_field = |label: &str, field: EditField, text: String, fx: f32, vx: f32, yref: &mut f32| {
        draw_text(label, fx, *yref, label_size, WHITE);
        let editing = state.editing_field == Some(field);
        let show = if editing { format!("{}_", state.input_buffer) } else { text };
        let color = if editing { YELLOW } else { Color::new(0.5, 0.9, 0.5, 1.0) };
        draw_text(&show, vx, *yref, value_size, color);
        if is_mouse_button_pressed(MouseButton::Left) {
            let (mx, my) = mouse_position();
            if mx >= vx && mx <= vx + 200.0 && my >= *yref - 20.0 && my <= *yref + 5.0 {
                state.editing_field = Some(field);
                state.input_buffer = show.trim_end_matches('_').to_string();
            }
        }
        *yref += line_h;
    };

    // Left column
    draw_field("World Width:", EditField::WorldWidth, format!("{:.0}", state.config.world_width), field_x1, value_x1, &mut y1);
    draw_field("World Height:", EditField::WorldHeight, format!("{:.0}", state.config.world_height), field_x1, value_x1, &mut y1);
    draw_field("Agents:", EditField::PopSize, format!("{}", state.config.population_size), field_x1, value_x1, &mut y1);
    draw_field("Initial Energy:", EditField::InitialEnergy, format!("{:.1}", state.config.initial_energy), field_x1, value_x1, &mut y1);
    draw_field("Max Energy:", EditField::MaxEnergy, format!("{:.1}", state.config.max_energy), field_x1, value_x1, &mut y1);
    draw_field("Drain/Step:", EditField::EnergyDrain, format!("{:.3}", state.config.energy_drain_per_step), field_x1, value_x1, &mut y1);

    // Right column
    draw_field("Max Food:", EditField::MaxFood, format!("{}", state.config.max_food), field_x2, value_x2, &mut y2);
    draw_field("Spawn Rate:", EditField::FoodRespawnRate, format!("{:.4}", state.config.food_respawn_prob), field_x2, value_x2, &mut y2);

    y = y1.max(y2) + 20.0;

    // Fitness Weights section (two columns)
    draw_text("Fitness Weights:", x, y, label_size, LIGHTGRAY);
    y += line_h;

    let mut draw_weight = |label: &str, field: EditField, val: f32, col_x: f32, wy: &mut f32| {
        draw_text(label, col_x, *wy, label_size, WHITE);
        let editing = state.editing_field == Some(field);
        let text = if editing { format!("{}_", state.input_buffer) } else { format!("{:.3}", val) };
        let color = if editing { YELLOW } else { Color::new(0.5, 0.9, 0.5, 1.0) };
        let vx = col_x + 170.0;
        draw_text(&text, vx, *wy, value_size, color);
        if is_mouse_button_pressed(MouseButton::Left) {
            let (mx, my) = mouse_position();
            if mx >= vx && mx <= vx + 200.0 && my >= *wy - 20.0 && my <= *wy + 5.0 {
                state.editing_field = Some(field);
                state.input_buffer = format!("{:.3}", val);
            }
        }
        *wy += line_h;
    };

    let mut wyl = y;
    let mut wyr = y;
    // Left column weights
    draw_weight("Lifetime:", EditField::WLifetime, state.config.w_lifetime, field_x1, &mut wyl);
    draw_weight("Energy:", EditField::WEnergy, state.config.w_energy, field_x1, &mut wyl);
    draw_weight("Offspring:", EditField::WOffspring, state.config.w_offspring, field_x1, &mut wyl);
    draw_weight("Comm:", EditField::WComm, state.config.w_comm, field_x1, &mut wyl);
    draw_weight("Idle Penalty:", EditField::WIdle, state.config.w_idle_penalty, field_x1, &mut wyl);
    draw_weight("Plants:", EditField::WPlant, state.config.w_plant, field_x1, &mut wyl);

    // Right column weights
    draw_weight("Meat:", EditField::WMeat, state.config.w_meat, field_x2, &mut wyr);
    draw_weight("Attacks:", EditField::WAttacks, state.config.w_attacks, field_x2, &mut wyr);
    draw_weight("Kills:", EditField::WKills, state.config.w_kills, field_x2, &mut wyr);
    draw_weight("Herding:", EditField::WHerd, state.config.w_herding, field_x2, &mut wyr);
    draw_weight("Approach:", EditField::WApproach, state.config.w_approach, field_x2, &mut wyr);
    draw_weight("Chase:", EditField::WChase, state.config.w_chase, field_x2, &mut wyr);
    draw_weight("Chase Same:", EditField::WChaseSame, state.config.w_chase_same, field_x2, &mut wyr);

    y = wyl.max(wyr) + 10.0;

    // Handle keyboard input for editing
    if let Some(field) = state.editing_field {
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
                    EditField::WLifetime => { state.config.w_lifetime = val; }
                    EditField::WEnergy => { state.config.w_energy = val; }
                    EditField::WOffspring => { state.config.w_offspring = val; }
                    EditField::WComm => { state.config.w_comm = val; }
                    EditField::WIdle => { state.config.w_idle_penalty = val.max(0.0); }
                    EditField::WPlant => { state.config.w_plant = val; }
                    EditField::WMeat => { state.config.w_meat = val; }
                    EditField::WAttacks => { state.config.w_attacks = val; }
                    EditField::WKills => { state.config.w_kills = val; }
                    EditField::WHerd => { state.config.w_herding = val; }
                    EditField::WApproach => { state.config.w_approach = val; }
                    EditField::WChase => { state.config.w_chase = val; }
                    EditField::WChaseSame => { state.config.w_chase_same = val; }
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
            if ch.is_ascii_digit() || ch == '.' || ch == '-' {
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
