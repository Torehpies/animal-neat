use macroquad::prelude::*;
use std::collections::VecDeque;
use std::path::PathBuf;

/// Result of the in-simulation modal menu
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimMenuResult {
    Resume,
    BackToMain,
}

/// Run the modal in-simulation menu. This function takes ownership of drawing and input
/// handling for the menu until the user selects an action. It mutates the provided AppState
/// for load/save operations.
pub async fn run_sim_menu(state: &mut crate::AppState) -> SimMenuResult {
    use crate::{snapshot, ui_load_picker, ui_save_picker, io};
    use crate::sim::{Agent, AgentId};
    use crate::body::Body;
    use crate::params::AGENT_RADIUS;

    let mut click_cooldown: f32 = 0.0;

    loop {
        clear_background(Color::new(0.0, 0.0, 0.0, 0.0));
        let w = screen_width();
        let h = screen_height();

        // Panel
        let panel_w = 420.0;
        let panel_h = 240.0;
        let cx = w * 0.5;
        let cy = h * 0.5;
        let panel_x = cx - panel_w * 0.5;
        let panel_y = cy - panel_h * 0.5;
        draw_rectangle(panel_x, panel_y, panel_w, panel_h, Color::new(0.06, 0.06, 0.08, 0.96));
        draw_rectangle_lines(panel_x, panel_y, panel_w, panel_h, 2.0, WHITE);

        // Title
        let title = "Paused — Simulation Menu";
        draw_text(title, panel_x + 18.0, panel_y + 36.0, 28.0, WHITE);

        // Buttons
        let left_x = panel_x + 24.0;
        let mut by = panel_y + 64.0;
        let btn_w = 124.0;
        let btn_h = 44.0;
        let gap = 12.0;

    let (mx, my) = mouse_position();

        // Resume
        let resume_x = left_x;
        let resume_y = by;
        let hovering_resume = mx >= resume_x && mx <= resume_x + btn_w && my >= resume_y && my <= resume_y + btn_h;
        let resume_col = if hovering_resume { Color::new(0.25, 0.7, 0.25, 1.0) } else { Color::new(0.18, 0.5, 0.18, 1.0) };
        draw_rectangle(resume_x, resume_y, btn_w, btn_h, resume_col);
        draw_rectangle_lines(resume_x, resume_y, btn_w, btn_h, 2.0, WHITE);
        let txt = "Resume";
        let tw = measure_text(txt, None, 20, 1.0).width;
        draw_text(txt, resume_x + (btn_w - tw) / 2.0, resume_y + 28.0, 20.0, WHITE);

        // Save
        let save_x = resume_x + btn_w + gap;
        let save_y = by;
        let hovering_save = mx >= save_x && mx <= save_x + btn_w && my >= save_y && my <= save_y + btn_h;
        let save_col = if hovering_save { Color::new(0.25, 0.6, 0.9, 1.0) } else { Color::new(0.15, 0.45, 0.75, 1.0) };
        draw_rectangle(save_x, save_y, btn_w, btn_h, save_col);
        draw_rectangle_lines(save_x, save_y, btn_w, btn_h, 2.0, WHITE);
        let txt = "Save";
        let tw = measure_text(txt, None, 20, 1.0).width;
        draw_text(txt, save_x + (btn_w - tw) / 2.0, save_y + 28.0, 20.0, WHITE);

        // Load
        let load_x = save_x + btn_w + gap;
        let load_y = by;
        let hovering_load = mx >= load_x && mx <= load_x + btn_w && my >= load_y && my <= load_y + btn_h;
        let load_col = if hovering_load { Color::new(0.9, 0.6, 0.25, 1.0) } else { Color::new(0.7, 0.45, 0.12, 1.0) };
        draw_rectangle(load_x, load_y, btn_w, btn_h, load_col);
        draw_rectangle_lines(load_x, load_y, btn_w, btn_h, 2.0, WHITE);
        let txt = "Load";
        let tw = measure_text(txt, None, 20, 1.0).width;
        draw_text(txt, load_x + (btn_w - tw) / 2.0, load_y + 28.0, 20.0, WHITE);

        by += btn_h + 18.0;

        // Back to Main Menu
        let back_x = left_x;
        let back_y = by;
        let back_w = panel_w - 64.0;
        let hovering_back = mx >= back_x && mx <= back_x + back_w && my >= back_y && my <= back_y + btn_h;
        let back_col = if hovering_back { Color::new(0.8, 0.25, 0.25, 1.0) } else { Color::new(0.6, 0.18, 0.18, 1.0) };
        draw_rectangle(back_x, back_y, back_w, btn_h, back_col);
        draw_rectangle_lines(back_x, back_y, back_w, btn_h, 2.0, WHITE);
        let txt = "Back to Main Menu";
        let tw = measure_text(txt, None, 20, 1.0).width;
        draw_text(txt, back_x + (back_w - tw) / 2.0, back_y + 28.0, 20.0, WHITE);

        // Input handling
        if is_mouse_button_pressed(MouseButton::Left) {
            if click_cooldown <= 0.0 {
                if hovering_resume {
                    click_cooldown = 0.25;
                    return SimMenuResult::Resume;
                } else if hovering_save {
                    // Close menu visually by returning to caller after save completes
                    click_cooldown = 0.20;
                    while click_cooldown > 0.0 { let dt = get_frame_time(); click_cooldown -= dt; next_frame().await; }
                    if let Some(path) = ui_save_picker::pick_save(None).await {
                        let _ = snapshot::save_sim_snapshot(&path, state.generation, &state.population, &state.innov, &state.episode, &state.member_species);
                    }
                    // continue showing menu after save
                } else if hovering_load {
                    click_cooldown = 0.20;
                    while click_cooldown > 0.0 { let dt = get_frame_time(); click_cooldown -= dt; next_frame().await; }
                    if let Some(path) = ui_load_picker::pick_snapshot().await {
                        // Attempt to load full sim snapshot, fallback to population-only
                        if let Ok(snap) = snapshot::load_sim_snapshot(&path) {
                            state.population = snap.population;
                            state.generation = snap.generation;
                            state.innov = snap.innovation;
                            state.episode.food = snap.food.iter().map(|v| v.to_vec2()).collect();
                            state.episode.agents.clear();
                            for (i, a_snap) in snap.agents.iter().enumerate() {
                                let body = Body { pos: a_snap.body_pos.to_vec2(), vel: a_snap.body_vel.to_vec2(), radius: AGENT_RADIUS };
                                state.episode.agents.push(Agent {
                                    id: AgentId(i),
                                    body,
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
                                    digest: VecDeque::new(),
                                    last_food_mem: a_snap.last_food_mem.to_vec2(),
                                    last_danger_mem: a_snap.last_danger_mem.to_vec2(),
                                    last_same_mem: a_snap.last_same_mem.to_vec2(),
                                    last_other_mem: a_snap.last_other_mem.to_vec2(),
                                    species_id: a_snap.species_id,
                                    age_steps: a_snap.age_steps,
                                    call_intensity: a_snap.call_intensity,
                                    heard_sectors: a_snap.heard_sectors,
                                    repro_cooldown: a_snap.repro_cooldown,
                                    offspring_count: a_snap.offspring_count,
                                    attack_hits: a_snap.attack_hits,
                                    kills_caused: a_snap.kills_caused,
                                    idle_anchor: a_snap.idle_anchor.to_vec2(),
                                    idle_steps: a_snap.idle_steps,
                                    total_idle_penalty: a_snap.total_idle_penalty,
                                });
                            }
                            state.episode.steps = snap.episode_steps;
                            state.speciator.get_species_mut().clear();
                            state.speciator.speciate(&state.population);
                            state.member_species = snap.member_species;
                        } else {
                            match io::load_population_snapshot(&path) {
                                Ok(snap) => {
                                    state.population = snap.population;
                                    state.generation = snap.generation;
                                    state.innov = snap.innovation;
                                    state.speciator.get_species_mut().clear();
                                    state.speciator.speciate(&state.population);
                                    state.member_species = {
                                        let mut map = vec![0usize; state.population.len()];
                                        for (sidx, s) in state.speciator.get_species().iter().enumerate() {
                                            for &m in &s.members { if m < state.population.len() { map[m] = sidx; } }
                                        }
                                        map
                                    };
                                    let mut rng = ::rand::rng();
                                    state.episode = crate::sim::Episode::new(&mut rng, state.population.len(), &state.member_species);
                                }
                                Err(_) => {}
                            }
                        }
                        // after loading, keep showing the menu
                    }
                } else if hovering_back {
                    return SimMenuResult::BackToMain;
                }
            }
        }

        // cooldown decrement
        if click_cooldown > 0.0 { click_cooldown = (click_cooldown - get_frame_time()).max(0.0); }

        next_frame().await;
    }
}
