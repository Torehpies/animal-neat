use macroquad::prelude::*;

/// Result of the in-simulation modal menu
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimMenuResult {
    Resume,
    BackToMain,
}

/// Run the simulation menu with full async support for Save/Load pickers.
/// This function handles the modal overlay and all button interactions.
pub async fn run_sim_menu(state: &mut crate::AppState) -> SimMenuResult {
    let mut click_cooldown = 0.0f32;

    loop {
        // Draw a semi-transparent dark overlay over the frozen simulation
        let w = screen_width();
        let h = screen_height();
        draw_rectangle(0.0, 0.0, w, h, Color::new(0.0, 0.0, 0.0, 0.7));

    // Panel (compute height to fit title + buttons)
    let panel_w: f32 = 460.0f32;
    let title = "Paused — Simulation Menu";
    let title_size: f32 = 28.0f32;

    // Button/layout sizing
    let btn_h: f32 = 48.0f32;
    let gap: f32 = 15.0f32;
    let num_buttons: f32 = 4.0f32; // Resume, Save, Load, Back
    let padding: f32 = 30.0f32;
    let title_gap: f32 = 20.0f32; // space between title and first button

    let buttons_h_total = num_buttons * btn_h + (num_buttons - 1.0) * gap;
    let panel_h = (padding + title_size + title_gap + buttons_h_total + padding).clamp(240.0f32, h - 60.0f32);

    let cx = w * 0.5;
    let cy = h * 0.5;
    let panel_x = cx - panel_w * 0.5;
    let panel_y = cy - panel_h * 0.5;
    draw_rectangle(panel_x, panel_y, panel_w, panel_h, Color::new(0.06, 0.06, 0.08, 0.96));
    draw_rectangle_lines(panel_x, panel_y, panel_w, panel_h, 2.0, WHITE);

    // Title (centered)
    let title_w = measure_text(title, None, title_size as u16, 1.0).width;
    draw_text(title, panel_x + (panel_w - title_w) / 2.0, panel_y + padding + title_size, title_size, WHITE);

    // Buttons (vertical stack, equal-sized)
    let mut by = panel_y + padding + title_size + title_gap;
    let btn_w = panel_w - 2.0 * padding; // full width within padding

    let (mx, my) = mouse_position();

    let start_x = panel_x + padding;

    let txt_size = 22.0;

    // Resume
    let resume_x = start_x;
    let resume_y = by;
    let hovering_resume = mx >= resume_x && mx <= resume_x + btn_w && my >= resume_y && my <= resume_y + btn_h;
    let resume_col = if hovering_resume { Color::new(0.25, 0.7, 0.25, 1.0) } else { Color::new(0.18, 0.5, 0.18, 1.0) };
    draw_rectangle(resume_x, resume_y, btn_w, btn_h, resume_col);
    draw_rectangle_lines(resume_x, resume_y, btn_w, btn_h, 2.0, WHITE);
    let txt_resume = "Resume";
    let tw_resume = measure_text(txt_resume, None, txt_size as u16, 1.0).width;
    draw_text(txt_resume, resume_x + (btn_w - tw_resume) / 2.0, resume_y + (btn_h + txt_size) / 2.0 - 4.0, txt_size, WHITE);

    by += btn_h + gap;

    // Save
    let save_x = start_x;
    let save_y = by;
    let hovering_save = mx >= save_x && mx <= save_x + btn_w && my >= save_y && my <= save_y + btn_h;
    let save_col = if hovering_save { Color::new(0.25, 0.6, 0.9, 1.0) } else { Color::new(0.15, 0.45, 0.75, 1.0) };
    draw_rectangle(save_x, save_y, btn_w, btn_h, save_col);
    draw_rectangle_lines(save_x, save_y, btn_w, btn_h, 2.0, WHITE);
    let txt_save = "Save";
    let tw_save = measure_text(txt_save, None, txt_size as u16, 1.0).width;
    draw_text(txt_save, save_x + (btn_w - tw_save) / 2.0, save_y + (btn_h + txt_size) / 2.0 - 4.0, txt_size, WHITE);

    by += btn_h + gap;

    // Load
    let load_x = start_x;
    let load_y = by;
    let hovering_load = mx >= load_x && mx <= load_x + btn_w && my >= load_y && my <= load_y + btn_h;
    let load_col = if hovering_load { Color::new(0.9, 0.6, 0.25, 1.0) } else { Color::new(0.7, 0.45, 0.12, 1.0) };
    draw_rectangle(load_x, load_y, btn_w, btn_h, load_col);
    draw_rectangle_lines(load_x, load_y, btn_w, btn_h, 2.0, WHITE);
    let txt_load = "Load";
    let tw_load = measure_text(txt_load, None, txt_size as u16, 1.0).width;
    draw_text(txt_load, load_x + (btn_w - tw_load) / 2.0, load_y + (btn_h + txt_size) / 2.0 - 4.0, txt_size, WHITE);

    by += btn_h + gap;

    // Back to Main Menu (same size)
    let back_x = start_x;
    let back_y = by;
    let back_w = btn_w;
    let hovering_back = mx >= back_x && mx <= back_x + back_w && my >= back_y && my <= back_y + btn_h;
    let back_col = if hovering_back { Color::new(0.8, 0.25, 0.25, 1.0) } else { Color::new(0.6, 0.18, 0.18, 1.0) };
    draw_rectangle(back_x, back_y, back_w, btn_h, back_col);
    draw_rectangle_lines(back_x, back_y, back_w, btn_h, 2.0, WHITE);
    let txt_back = "Back to Main Menu";
    let tw_back = measure_text(txt_back, None, txt_size as u16, 1.0).width;
    draw_text(txt_back, back_x + (back_w - tw_back) / 2.0, back_y + (btn_h + txt_size) / 2.0 - 4.0, txt_size, WHITE);

        // Input handling
        if is_mouse_button_pressed(MouseButton::Left) && click_cooldown <= 0.0 {
            if hovering_resume {
                //click_cooldown = 0.15;
                return SimMenuResult::Resume;
            } else if hovering_save && click_cooldown <= 0.0 {
                // Handle Save: show save picker
                //click_cooldown = 0.20;
                // Wait for cooldown
                let cooldown_start = get_time();
                while get_time() - cooldown_start < 0.20 {
                    next_frame().await;
                }
                
                if let Some(save_path) = crate::ui_save_picker::pick_save(None).await {
                    // Save the current simulation state
                    if let Err(e) = crate::snapshot::save_sim_snapshot(
                        &save_path,
                        state.generation,
                        &state.population,
                        &state.innov,
                        &state.episode,
                        &state.member_species,
                    ) {
                        eprintln!("Failed to save snapshot: {}", e);
                    } else {
                        println!("Saved simulation to {}", save_path);
                    }
                }
                // Reset cooldown after picker closes
                click_cooldown = 0.15;
            } else if hovering_load && click_cooldown <= 0.0 {
                // Handle Load: show load picker
                //click_cooldown = 0.20;
                // Wait for cooldown
                let cooldown_start = get_time();
                while get_time() - cooldown_start < 0.20 {
                    next_frame().await;
                }
                
                if let Some(load_path) = crate::ui_load_picker::pick_snapshot().await {
                    // Load the simulation state
                    match crate::snapshot::load_sim_snapshot(&load_path) {
                        Ok(snap) => {
                            state.population = snap.population;
                            state.generation = snap.generation;
                            state.innov = snap.innovation;
                            state.member_species = snap.member_species;
                            
                            // Rebuild episode from snapshot
                            use crate::sim::{Agent, AgentId};
                            use crate::body::Body;
                            use crate::params::AGENT_COLLISION_RADIUS;
                            
                            let mut new_agents: Vec<Agent> = Vec::new();
                            for a_snap in &snap.agents {
                                new_agents.push(Agent {
                                    id: AgentId(new_agents.len()),
                                    kind: a_snap.kind,
                                    body: Body {
                                        pos: a_snap.body_pos.to_vec2(),
                                        vel: a_snap.body_vel.to_vec2(),
                                        radius: AGENT_COLLISION_RADIUS,
                                    },
                                    theta: a_snap.theta,
                                    energy: a_snap.energy,
                                    health: a_snap.health,
                                    max_health: a_snap.max_health,
                                    invuln_steps: a_snap.invuln_steps,
                                    alive_steps: a_snap.alive_steps,
                                    eaten: a_snap.eaten,
                                    consumed: a_snap.consumed,
                                    kills: a_snap.kills,
                                    predation_flash_steps: a_snap.predation_flash_steps,
                                    dead_since: a_snap.dead_since,
                                    corpse_energy: a_snap.corpse_energy,
                                    digest: std::collections::VecDeque::new(), // Reset digest queue
                                    last_food_mem: a_snap.last_food_mem.to_vec2(),
                                    last_danger_mem: a_snap.last_danger_mem.to_vec2(),
                                    last_same_mem: a_snap.last_same_mem.to_vec2(),
                                    last_other_mem: a_snap.last_other_mem.to_vec2(),
                                    species_id: a_snap.species_id,
                                    age_steps: a_snap.age_steps,
                                    attack_hits: a_snap.attack_hits,
                                    kills_caused: a_snap.kills_caused,
                                    call_intensity: a_snap.call_intensity,
                                    heard_sectors: a_snap.heard_sectors,
                                    repro_cooldown: a_snap.repro_cooldown,
                                    offspring_count: a_snap.offspring_count,
                                    idle_anchor: a_snap.idle_anchor.to_vec2(),
                                    idle_steps: a_snap.idle_steps,
                                    total_idle_penalty: a_snap.total_idle_penalty,
                                    total_idle_steps: 0,
                                    input_buf: Vec::new(),
                                    energy_accum: 0.0,
                                    herding_units: 0.0,
                                    approach_food_units: 0.0,
                                    chase_other_units: 0.0,
                                    chase_same_units: 0.0,
                                    flee_other_units: 0.0,
                                    hunger: 0.0,
                                    contentment: 1.0,
                                    rest_content_units: 0.0,
                                    eat_early_units: 0.0,
                                    is_eating: false,
                                });
                            }
                            
                            state.episode.food = snap.food.iter().map(|v| v.to_vec2()).collect();
                            state.episode.agents = new_agents;
                            state.episode.steps = snap.episode_steps;
                            
                            state.speciator.speciate(&state.population);
                            
                            println!("Loaded simulation from {}", load_path);
                        }
                        Err(e) => {
                            eprintln!("Failed to load snapshot: {}", e);
                        }
                    }
                }
                // Reset cooldown after picker closes
                click_cooldown = 0.15;
            } else if hovering_back {
                //click_cooldown = 0.20;
                // Wait for cooldown
                let cooldown_start = get_time();
                while get_time() - cooldown_start < 0.20 {
                    next_frame().await;
                }
                return SimMenuResult::BackToMain;
            }
        }

        // Cooldown decrement
        if click_cooldown > 0.0 {
            click_cooldown = (click_cooldown - get_frame_time()).max(0.0);
        }

        next_frame().await;
    }
}
