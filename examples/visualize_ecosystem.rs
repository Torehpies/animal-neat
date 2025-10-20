//! Visualize Ecosystem example
//!
//! High-level flow:
//! - AppState holds the evolving NEAT population plus visualization flags.
//! - In live mode, an Episode advances step-by-step and the UI renders agents, overlays, and HUD.
//!
//! Inputs: see params.rs for the fixed layout; use `sensing::input_ranges()` for indices.
//! Outputs: [turn, thrust, call]. Movement uses an inertia model when enabled.
//!
//! Key files:
//! - params.rs: all configuration
//! - sensing.rs: input building (distance-based vision), hearing
//! - sim.rs: movement & interactions (predation/scavenging)
//! - world.rs: plant growth and seasonality
//! - ui/: world view, HUD, network panel

use macroquad::prelude::*;
use neat::neat::{
    config::EvolutionConfig,
    evolution,
    genome::Genome,
    innovation_tracker::InnovationTracker,
};
#[path = "visualize_ecosystem/params.rs"]
mod params;
#[path = "visualize_ecosystem/sensing.rs"]
mod sensing;
#[path = "visualize_ecosystem/world.rs"]
mod world;
#[path = "visualize_ecosystem/ui/common.rs"]
mod ui_common;
#[path = "visualize_ecosystem/ui/world_view.rs"]
mod ui_world_view;
#[path = "visualize_ecosystem/ui/hud.rs"]
mod ui_hud;
#[path = "visualize_ecosystem/ui/network.rs"]
mod ui_network;
#[path = "visualize_ecosystem/ui/menu_backend.rs"]
mod menu_backend;
#[path = "visualize_ecosystem/ui/menu.rs"]
mod ui_menu;
#[path = "visualize_ecosystem/ui/main_menu.rs"]
mod ui_main_menu;
#[path = "visualize_ecosystem/ui/graphs.rs"]
mod ui_graphs;
#[path = "visualize_ecosystem/elements/body.rs"]
mod body;
#[path = "visualize_ecosystem/ui/scoreboard.rs"]
mod ui_scoreboard;
#[path = "visualize_ecosystem/scoreboard.rs"]
mod scoreboard;
#[path = "visualize_ecosystem/sim/mod.rs"]
mod sim;
#[path = "visualize_ecosystem/snapshot.rs"]
mod snapshot;
#[path = "visualize_ecosystem/ui/load_picker.rs"]
mod ui_load_picker;
#[path = "visualize_ecosystem/ui/save_picker.rs"]
mod ui_save_picker;
#[path = "visualize_ecosystem/ui/sim_menu.rs"]
mod ui_sim_menu;
#[path = "visualize_ecosystem/ui/controls.rs"]
mod ui_controls;
#[path = "visualize_ecosystem/ui/assets.rs"]
mod ui_assets;
use sim::{Episode, Agent, AgentId};
use ::rand::Rng;
use params::*;
// eval_population_single_episode is used by AppState in app_state.rs

#[path = "visualize_ecosystem/app_state.rs"]
mod app_state;
pub use app_state::AppState;

// Episode methods are defined in sim::episode

use ui_common::screen_to_world;

// (Body and wrap_to_world are used inside sim/ui modules)

// draw_world moved to ui_world_view::draw_world

// draw_hud moved to ui_hud::draw_hud

// draw_network_panel moved to ui_network::draw_network_panel

fn window_conf() -> Conf {
    Conf {
        window_title: "NEAT Ecosystem Visualizer".to_string(),
        fullscreen: true,
        window_width: 1280,
        window_height: 800,
        high_dpi: true,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    // Top-level loop so we can return to the main menu (label used by ESC handler)
    loop {
    // Main menu runner (in separate module) - returns when user chooses to create a sim, load one, or exits
        let menu_res = ui_main_menu::run_main_menu().await;
        let (sim_config, loaded_snapshot_path) = match menu_res {
            ui_main_menu::MenuResult::New(cfg) => (cfg, None),
            ui_main_menu::MenuResult::Exit => return,
            ui_main_menu::MenuResult::Load(path) => (ui_menu::SimConfig::default(), Some(path)),
        };
        
        // Apply configuration to global params (via world module)
        world::set_runtime_config(sim_config.world_width, sim_config.world_height, sim_config.max_food, sim_config.food_respawn_prob);
        params::set_runtime_energy_config_per_kind(
            sim_config.initial_energy_herb,
            sim_config.max_energy_herb,
            sim_config.energy_drain_per_step_herb,
            sim_config.initial_energy_carn,
            sim_config.max_energy_carn,
            sim_config.energy_drain_per_step_carn,
        );
        params::set_runtime_population_size(sim_config.population_size);
        // Apply fitness weights before starting
        params::set_fitness_weights_per_kind(
            // herbivore
            sim_config.w_lifetime_herb,
            sim_config.w_energy_herb,
            sim_config.w_offspring_herb,
            sim_config.w_comm_herb,
            sim_config.w_idle_penalty_herb,
            sim_config.w_plant_herb,
            sim_config.w_meat_herb,
            sim_config.w_attacks_herb,
            sim_config.w_kills_herb,
            sim_config.w_herding_herb,
            // carnivore
            sim_config.w_lifetime_carn,
            sim_config.w_energy_carn,
            sim_config.w_offspring_carn,
            sim_config.w_comm_carn,
            sim_config.w_idle_penalty_carn,
            sim_config.w_plant_carn,
            sim_config.w_meat_carn,
            sim_config.w_attacks_carn,
            sim_config.w_kills_carn,
            sim_config.w_herding_carn,
        );
        params::set_behavior_weights_per_kind(
            // herbivore
            sim_config.w_approach_herb,
            sim_config.w_chase_herb,
            sim_config.w_chase_same_herb,
            // carnivore
            sim_config.w_approach_carn,
            sim_config.w_chase_carn,
            sim_config.w_chase_same_carn,
        );
        
        let mut state = AppState::new(sim_config);
        // Preload textures (optional); use new assets helper which tries common locations.
        let assets = ui_assets::preload_textures().await;
        state.herb_tex = assets.herb;
        state.carn_tex = assets.carn;
        state.plant_tex = assets.plant;
        state.meat_tex = assets.meat;
        if state.herb_tex.is_none() { eprintln!("Warning: herbivore sprite not found (assets/sheep.png or .vscode/assets/sheep.png)"); }
        if state.carn_tex.is_none() { eprintln!("Warning: carnivore sprite not found (assets/wolf.png or .vscode/assets/wolf.png)"); }
        if state.plant_tex.is_none() { eprintln!("Warning: plant sprite not found (assets/plant_1.png or .vscode/assets/plant_1.png)"); }
        if state.meat_tex.is_none() { eprintln!("Warning: meat sprite not found (assets/meat.png or .vscode/assets/meat.png)"); }
        // If the main menu requested to load a snapshot, apply it now
        if let Some(path) = loaded_snapshot_path {
            if let Ok(snap) = snapshot::load_sim_snapshot(&path) {
                state.population = snap.population;
                state.generation = snap.generation;
                state.innov = snap.innovation;
                state.episode.food = snap.food.iter().map(|v| v.to_vec2()).collect();
                state.episode.agents.clear();
                for (i, a_snap) in snap.agents.iter().enumerate() {
                    let body = crate::body::Body { pos: a_snap.body_pos.to_vec2(), vel: a_snap.body_vel.to_vec2(), radius: AGENT_COLLISION_RADIUS };
                    state.episode.agents.push(Agent {
                        id: AgentId(i),
                        kind: a_snap.kind,
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
                        digest: std::collections::VecDeque::new(),
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
                        total_idle_steps: 0,
                        input_buf: Vec::new(),
                        energy_accum: 0.0,
                        herding_units: 0.0,
                        approach_food_units: 0.0,
                        chase_other_units: 0.0,
                        chase_same_units: 0.0,
                    });
                }
                state.episode.steps = snap.episode_steps;
                state.speciator.get_species_mut().clear();
                state.speciator.speciate(&state.population);
                state.member_species = snap.member_species;
                println!("Loaded sim snapshot from {}", path);
            } else {
                eprintln!("Failed to load snapshot from {}", path);
            }
        }
        let mut running = true;      // continuous evolution by default
        let mut fast_mode = false;   // start at normal speed
    // In-simulation modal handled by ui_sim_menu
        // Click cooldown (seconds) to avoid double/triple activations from fast clicks
        let mut click_cooldown: f32 = 0.0;
        let mut normal_step_timer = 0.0f32;          // accumulates frame time for normal stepping
        let normal_step_interval = 0.05f32;           // seconds per simulation step in normal mode
        let fast_steps_per_frame: usize = 500;       // simulation steps per frame in fast mode

        'sim_loop: loop {
            clear_background(BLACK);
            let w = screen_width();
            let h = screen_height();
            let margin = 16.0;
            let hud_w = (w * 0.28).clamp(240.0, 380.0);
            let world_w = (w - hud_w - margin * 3.0).max(100.0);
            let world_h = (h - margin * 2.0).max(100.0);
            let world_area = Rect { x: margin, y: margin, w: world_w, h: world_h };
            let hud_area = Rect { x: world_area.x + world_area.w + margin, y: margin, w: hud_w, h: world_h };

        // Controls
            match ui_controls::handle_keyboard(&mut state, &mut running, &mut fast_mode, &mut click_cooldown) {
                ui_controls::ControlsAction::None => {}
                ui_controls::ControlsAction::OpenMenu => {
                    let prev_running_state = running;
                    match ui_sim_menu::run_sim_menu(&mut state).await {
                        ui_sim_menu::SimMenuResult::Resume => { running = prev_running_state; }
                        ui_sim_menu::SimMenuResult::BackToMain => { break 'sim_loop; }
                    }
                }
                ui_controls::ControlsAction::FinalizeEndOfEpisode => {
                    let mut rng = ::rand::rng();
                    finalize_end_of_episode(&mut state, &mut rng);
                    running = true;
                }
            }
    // 'O' key: (removed) FPS counter is always displayed now
        
        // ESC: if focused on an agent, clear focus; otherwise open the in-sim menu
        if is_key_pressed(KeyCode::Escape) {
            if state.focused_agent.is_some() {
                state.focused_agent = None;
            } else {
                // Pause and show the overlay menu
                let prev_running_state = running;
                
                // Call the async menu - it handles its own rendering loop
                match ui_sim_menu::run_sim_menu(&mut state).await {
                    ui_sim_menu::SimMenuResult::Resume => {
                        running = prev_running_state;
                    }
                    ui_sim_menu::SimMenuResult::BackToMain => {
                        // return to main menu by exiting the simulation loop
                        break 'sim_loop;
                    }
                }
            }
        }
        // Removed per-row overlay toggles (1..4). Unified overlay is controlled via 'U'.
        // Removed: [S] save snapshot and [B] easy birth debug toggle

        // If scoreboard is open (episode ended and paused), block simulation/evolution
        if state.scoreboard_pending { running = false; }

        if running {
            let mut rng = ::rand::rng();
            if state.ultra_mode {
                // Ultra mode: headless-ish fast stepping, minimal sampling & no rendering until toggle off
                let steps_per_frame = 2000usize; // very high throughput
                for _ in 0..steps_per_frame {
                    if state.episode.is_finished() {
                        // At episode end: either open scoreboard (if toggle ON) or immediately continue (autoplay)
                        if state.show_scoreboard_panel {
                            if !state.scoreboard_pending { scoreboard::prepare_scoreboard(&mut state); }
                        } else {
                            finalize_end_of_episode(&mut state, &mut rng);
                        }
                        break;
                    }
                    state.episode.step(&state.population, &mut rng);
                    // Sparse sampling every 20 steps
                    if state.episode.steps % 20 == 0 {
                        let alive_ct = state.episode.agents.iter().filter(|a| a.energy > 0.0).count() as f32;
                        let species_ct = state.speciator.get_species().len() as f32;
                        let deaths_ct = state.episode.agents.iter().filter(|a| a.energy <= 0.0 && !a.consumed).count() as f32;
                        // Per-kind deaths
                        let deaths_herb = state.episode.agents.iter().filter(|a| matches!(a.kind, crate::sim::AgentKind::Herbivore) && a.energy <= 0.0 && !a.consumed).count() as f32;
                        let deaths_carn = state.episode.agents.iter().filter(|a| matches!(a.kind, crate::sim::AgentKind::Carnivore) && a.energy <= 0.0 && !a.consumed).count() as f32;
                        state.graphs.pop.push(alive_ct);
                        state.graphs.species.push(species_ct);
                        state.graphs.births.push(state.episode.births_this_episode as f32);
                        state.graphs.deaths.push(deaths_ct);
                        state.graphs.births_herb.push(state.episode.births_herb as f32);
                        state.graphs.births_carn.push(state.episode.births_carn as f32);
                        state.graphs.deaths_herb.push(deaths_herb);
                        state.graphs.deaths_carn.push(deaths_carn);
                    }
                    let _ = spawn_offspring_if_needed(
                        &mut state.population,
                        &mut state.episode,
                        &mut state.innov,
                        &state.cfg,
                        &mut rng,
                    );
                }
                // handled inside loop above
            } else if fast_mode {
                // Run many simulation steps per frame until the episode finishes, then evolve
                for _ in 0..fast_steps_per_frame {
                    if state.episode.is_finished() {
                        if state.show_scoreboard_panel {
                            if !state.scoreboard_pending { scoreboard::prepare_scoreboard(&mut state); }
                        } else {
                            finalize_end_of_episode(&mut state, &mut rng);
                        }
                        break;
                    }
                    state.episode.step(&state.population, &mut rng);
                    // Update graphs trends per step
                    let alive_ct = state.episode.agents.iter().filter(|a| a.energy > 0.0).count() as f32;
                    let species_ct = state.speciator.get_species().len() as f32;
                    let deaths_ct = state.episode.agents.iter().filter(|a| a.energy <= 0.0 && !a.consumed).count() as f32;
                    let deaths_herb = state.episode.agents.iter().filter(|a| matches!(a.kind, crate::sim::AgentKind::Herbivore) && a.energy <= 0.0 && !a.consumed).count() as f32;
                    let deaths_carn = state.episode.agents.iter().filter(|a| matches!(a.kind, crate::sim::AgentKind::Carnivore) && a.energy <= 0.0 && !a.consumed).count() as f32;
                    state.graphs.pop.push(alive_ct);
                    state.graphs.species.push(species_ct);
                    state.graphs.births.push(state.episode.births_this_episode as f32);
                    state.graphs.deaths.push(deaths_ct);
                    state.graphs.births_herb.push(state.episode.births_herb as f32);
                    state.graphs.births_carn.push(state.episode.births_carn as f32);
                    state.graphs.deaths_herb.push(deaths_herb);
                    state.graphs.deaths_carn.push(deaths_carn);
                        if ECO_CONTINUOUS {
                            let _ = spawn_offspring_if_needed(
                                &mut state.population,
                                &mut state.episode,
                                &mut state.innov,
                                &state.cfg,
                                &mut rng,
                            );
                        }
                }
                // If scoreboard is off, evolution/next-episode is handled inline above (autoplay)
            } else {
                // Normal mode: advance one simulation step per second
                normal_step_timer += get_frame_time();
                if normal_step_timer >= normal_step_interval {
                    normal_step_timer -= normal_step_interval;
                    if !state.episode.is_finished() {
                        state.episode.step(&state.population, &mut rng);
                        // Update graphs per step
                        let alive_ct = state.episode.agents.iter().filter(|a| a.energy > 0.0).count() as f32;
                        let species_ct = state.speciator.get_species().len() as f32;
                        let deaths_ct = state.episode.agents.iter().filter(|a| a.energy <= 0.0 && !a.consumed).count() as f32;
                        let deaths_herb = state.episode.agents.iter().filter(|a| matches!(a.kind, crate::sim::AgentKind::Herbivore) && a.energy <= 0.0 && !a.consumed).count() as f32;
                        let deaths_carn = state.episode.agents.iter().filter(|a| matches!(a.kind, crate::sim::AgentKind::Carnivore) && a.energy <= 0.0 && !a.consumed).count() as f32;
                        state.graphs.pop.push(alive_ct);
                        state.graphs.species.push(species_ct);
                        state.graphs.births.push(state.episode.births_this_episode as f32);
                        state.graphs.deaths.push(deaths_ct);
                        state.graphs.births_herb.push(state.episode.births_herb as f32);
                        state.graphs.births_carn.push(state.episode.births_carn as f32);
                        state.graphs.deaths_herb.push(deaths_herb);
                        state.graphs.deaths_carn.push(deaths_carn);
                        if ECO_CONTINUOUS {
                            let _ = spawn_offspring_if_needed(
                                &mut state.population,
                                &mut state.episode,
                                &mut state.innov,
                                &state.cfg,
                                &mut rng,
                            );
                        }
                        if state.episode.is_finished() {
                if state.show_scoreboard_panel {
                    if !state.scoreboard_pending { scoreboard::prepare_scoreboard(&mut state); }
                            } else {
                                finalize_end_of_episode(&mut state, &mut rng);
                            }
                        }
                    } else {
                        // Episode already finished when we got here (very short episodes): handle boundary
                        if state.show_scoreboard_panel {
                            if !state.scoreboard_pending { scoreboard::prepare_scoreboard(&mut state); }
                        } else {
                            finalize_end_of_episode(&mut state, &mut rng);
                        }
                    }
                }
            }
        }

    // Mouse position in world-space (for focus and overlays) if inside world rect
    let (mx, my) = mouse_position();
    // Fit the world rect so aspect ratio is preserved
    let fitted = ui_common::fit_world_rect(world_area);
    let mouse_world = if mx >= fitted.x && mx <= fitted.x + fitted.w && my >= fitted.y && my <= fitted.y + fitted.h {
        Some(screen_to_world(fitted, mx, my))
    } else { None };

    // Mouse-driven focus handling
    ui_controls::handle_mouse_buttons(&mut state, mouse_world);
    // (mouse_world already defined above)

    if !state.ultra_mode {
        ui_world_view::draw_world(
            world_area,
            &state.episode,
            state.show_cones,
            &state.member_species,
            state.show_unified_overlay,
            mouse_world,
            state.show_energy_overlay,
            state.show_collision_radii,
            state.focused_agent,
            state.show_grid,
            true,  // vision grid
            false, // hearing disabled
            true,  // memory vectors
            state.color_by_species,
            state.herb_tex.as_ref(),
            state.carn_tex.as_ref(),
                state.plant_tex.as_ref(),
                state.meat_tex.as_ref(),
        );
    ui_hud::draw_hud(hud_area, &mut state, &mut running, &mut fast_mode);
        // Draw graphs overlay on top of HUD/world when enabled
        if state.show_graphs_overlay {
            let fullscreen = Rect { x: 0.0, y: 0.0, w, h };
            // Compute current herbivore/carnivore counts for the pie chart
            let mut herb = 0usize; let mut carn = 0usize;
            for a in &state.episode.agents {
                if a.energy > 0.0 && a.health > crate::params::DEATH_HEALTH_THRESHOLD && !a.consumed {
                    match a.kind { crate::sim::AgentKind::Herbivore => herb += 1, crate::sim::AgentKind::Carnivore => carn += 1 }
                }
            }
            ui_graphs::draw_graphs_overlay(fullscreen, &state.graphs, &mut state.graphs_tab, (herb, carn), &mut state.show_graphs_overlay);
        }
        // Scoreboard panel: shown after episodes only when toggle is ON
        if state.scoreboard_pending && state.show_scoreboard_panel {
            let fullscreen = Rect { x: 0.0, y: 0.0, w, h };
            let clicked = ui_scoreboard::draw_scoreboard(fullscreen, &state, &state.scoreboard_rows, true);
            if clicked { // Treat button click as continue too
                let mut rng = ::rand::rng();
                finalize_end_of_episode(&mut state, &mut rng);
                running = true;
            }
        }
    } else {
        // Minimal overlay text
        let txt = format!("ULTRA mode: steps {} | pop {} | births {}", state.episode.steps, state.population.len(), state.episode.births_this_episode);
        draw_text(&txt, 20.0, 26.0, 24.0, WHITE);
        draw_text("[X] Exit Ultra", 20.0, 54.0, 18.0, GRAY);
    }
    // Decrement click cooldown
    if click_cooldown > 0.0 {
        click_cooldown -= get_frame_time();
        if click_cooldown < 0.0 { click_cooldown = 0.0; }
    }

    // Tick HUD toast timer
    ui_assets::tick_hud_toast(&mut state);

    // modal menu handled by ui_sim_menu when ESC is pressed

        next_frame().await;
    } // end 'sim_loop
    } // end 'main_loop
}

// Scoreboard helpers moved to `scoreboard.rs`

fn finalize_end_of_episode(state: &mut AppState, rng: &mut impl ::rand::Rng) {
    use crate::params::ECO_CONTINUOUS;
    // Compute intelligence proxy (from the just-finished episode) before resetting
    // Proxy focuses on behavior-shaping components: herding + approach/chase (other/same)
    let (intel_best, intel_mean) = {
        // Use per-kind behavior shaping weights
        let mut best = f32::NEG_INFINITY;
        let mut sum = 0.0f32;
        let mut count = 0usize;
        for a in &state.episode.agents {
            let kind = match a.kind { crate::sim::AgentKind::Herbivore => crate::params::Kind::Herb, crate::sim::AgentKind::Carnivore => crate::params::Kind::Carn };
            let w_approach = crate::params::get_fit_approach_food_weight(kind);
            let w_chase = crate::params::get_fit_chase_other_weight(kind);
            let w_chase_same = crate::params::get_fit_chase_same_weight(kind);
            let w_herd = crate::params::get_fit_herding_weight(kind);
            let v = w_herd * a.herding_units
                + w_approach * a.approach_food_units
                + w_chase * a.chase_other_units
                + w_chase_same * a.chase_same_units;
            if v.is_finite() {
                if v > best { best = v; }
                sum += v; count += 1;
            }
        }
        let mean = if count > 0 { sum / count as f32 } else { 0.0 };
        (best, mean)
    };
    if ECO_CONTINUOUS {
        state.eco_episode_counter += 1;
        eco_cull_population_by_fitness(state);
        // Log fitness (best/mean) for eco mode as well
        if state.last_best.is_finite() { state.graphs.best.push(state.last_best); }
        if state.last_avg.is_finite() { state.graphs.mean.push(state.last_avg); }
        // Log intelligence proxy for this episode
        if intel_best.is_finite() { state.graphs.intel_best.push(intel_best); }
        if intel_mean.is_finite() { state.graphs.intel_mean.push(intel_mean); }
        state.graphs.reset_episode();
    state.episode = Episode::new(rng, state.population.len());
    } else {
        // Log intelligence proxy for this episode (before resetting)
        if intel_best.is_finite() { state.graphs.intel_best.push(intel_best); }
        if intel_mean.is_finite() { state.graphs.intel_mean.push(intel_mean); }
        state.evolve_one_generation();
        state.graphs.reset_episode();
    state.episode = Episode::new(rng, state.population.len());
        if state.last_best.is_finite() { state.graphs.best.push(state.last_best); state.graphs.mean.push(state.last_avg); }
    }
    state.scoreboard_pending = false;
    // Keep panel toggle as the user last set it (don't force-close), but clear the data
    state.scoreboard_rows.clear();
}

fn eco_cull_population_by_fitness(state: &mut AppState) {
    // Strict NEAT at episode end using live-episode fitness for ALL agents (including newborns).
    // 1) Compute fitness scores from the finished episode
    let mut scores: Vec<f32> = {
        let complexity_penalty = crate::params::COMPLEXITY_PENALTY_PER_CONN;
        state.episode.agents.iter().enumerate().map(|(i, a)| {
            let kind = match a.kind { crate::sim::AgentKind::Herbivore => crate::params::Kind::Herb, crate::sim::AgentKind::Carnivore => crate::params::Kind::Carn };
            let w_life = crate::params::get_fit_lifetime_weight(kind);
            let w_energy = crate::params::get_fit_energy_weight(kind);
            let w_off = crate::params::get_fit_offspring_weight(kind);
            let w_comm = crate::params::get_fit_comm_weight(kind);
            let w_idle = crate::params::get_fit_idle_penalty_weight(kind);
            let w_plant = crate::params::get_fit_plant_weight(kind);
            let w_meat = crate::params::get_fit_meat_weight(kind);
            let w_att = crate::params::get_fit_attacks_weight(kind);
            let w_kill = crate::params::get_fit_kills_weight(kind);
            let w_herd = crate::params::get_fit_herding_weight(kind);
            let w_approach = crate::params::get_fit_approach_food_weight(kind);
            let w_chase = crate::params::get_fit_chase_other_weight(kind);
            let w_chase_same = crate::params::get_fit_chase_same_weight(kind);
            let lifetime_score = (a.alive_steps as f32) / (MAX_STEPS as f32);
            let avg_energy_norm = if a.alive_steps > 0 { (a.energy_accum / a.alive_steps as f32) / crate::params::get_max_energy_for(kind) } else { 0.0 };
            let offspring_score = a.offspring_count as f32;
            let comm_score = state.episode.comm_fitness_accum.get(i).copied().unwrap_or(0.0);
            // Normalize idle penalty to [0,1] of max achievable this episode
            let max_idle_steps = (MAX_STEPS.saturating_sub(IDLENESS_THRESHOLD_STEPS)) as f32;
            let max_idle_penalty = max_idle_steps * IDLENESS_PENALTY_PER_STEP;
            let idle_penalty = if max_idle_penalty > 0.0 { (a.total_idle_penalty / max_idle_penalty).clamp(0.0, 1.0) } else { 0.0 };
            let plants = a.eaten.saturating_sub(a.kills) as f32;
            let meat = a.kills as f32;
            let approach_units = a.approach_food_units;
            let chase_units = a.chase_other_units;
            let chase_same_units = a.chase_same_units;
            let mut s = w_life * lifetime_score
                + w_energy * avg_energy_norm
                + w_off * offspring_score
                + w_comm * comm_score
                - if IDLENESS_PENALTY_ENABLED { w_idle * idle_penalty } else { 0.0 }
                + w_plant * plants
                + w_meat * meat
                + w_att * (a.attack_hits as f32)
                + w_kill * (a.kills_caused as f32)
                + w_herd * a.herding_units
                + w_approach * approach_units
                + w_chase * chase_units
                + w_chase_same * chase_same_units;
            if complexity_penalty > 0.0 {
                let enabled = state.population[i].connections.iter().filter(|c| c.enabled).count() as f32;
                s -= complexity_penalty * enabled;
            }
            s
        }).collect()
    };

    // 2) Cull to target population size by global fitness order (if current pop > target)
    let target = crate::params::get_population_size();
    let current_len = state.population.len();
    if current_len > target {
        let mut idxs: Vec<usize> = (0..current_len).collect();
        idxs.sort_by(|&a, &b| scores[b]
            .partial_cmp(&scores[a])
            .unwrap_or(std::cmp::Ordering::Equal));
        idxs.truncate(target);
        // rebuild population and scores in selected order
        let mut new_pop = Vec::with_capacity(idxs.len());
        let mut new_scores = Vec::with_capacity(idxs.len());
        for i in idxs { new_pop.push(state.population[i].clone()); new_scores.push(scores[i]); }
        state.population = new_pop;
        scores = new_scores;
    }

    // 3) Hand off to the library's NEAT evolution with speciation-aware reproduction
    let old_len = state.population.len(); // after culling
    state.population = evolution::evolution(
        std::mem::take(&mut state.population),
        scores.clone(), // aligned 1:1 with culled population
        &mut state.speciator,
        &mut state.innov,
        &state.cfg,
    );
    debug_assert_eq!(state.population.len(), old_len, "population size should be stable across generations");

    // 4) Update stats for HUD
    if !scores.is_empty() {
        let best = scores.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        let avg = scores.iter().sum::<f32>() / (scores.len() as f32);
        state.last_best = best;
        state.last_avg = avg;
        state.last_best_generation = state.eco_episode_counter; // last eco-episode index
        // Snapshot best genome: pick elite after evolution (first of best species kept by evolution)
        if let Some((_idx, _)) = scores
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
        {
            // Note: after evolution, ordering changed; snapshot just uses the new population's first elite of the best species
            state.last_best_genome = state.population.first().cloned();
        }
    }

    // 5) Recompute species and member mapping for the next episode
    state.speciator.speciate(&state.population);
    state.member_species = {
        let mut map = vec![0usize; state.population.len()];
        for (sidx, s) in state.speciator.get_species().iter().enumerate() {
            for &m in &s.members {
                if m < state.population.len() {
                    map[m] = sidx;
                }
            }
        }
        map
    };
}

fn spawn_offspring_if_needed<R: Rng>(
    population: &mut Vec<Genome>,
    episode: &mut Episode,
    _innov: &mut InnovationTracker,
    _cfg: &EvolutionConfig,
    rng: &mut R,
) -> usize {
    // Count live agents and skip if at cap
    let mut live_indices: Vec<usize> = Vec::new();
    for (i, a) in episode.agents.iter().enumerate() {
        if a.energy > 0.0 && a.health > DEATH_HEALTH_THRESHOLD && !a.consumed { live_indices.push(i); }
    }
    if live_indices.len() >= ECO_MAX_POP { return 0; }

    // First, tick down cooldowns for all alive agents
    for i in &live_indices {
        let a = &mut episode.agents[*i];
        if a.repro_cooldown > 0 { a.repro_cooldown -= 1; }
    }

    // Gather eligible parents (energy, cooldown, offspring cap, optional non-idle)
    let mut eligible: Vec<usize> = Vec::new();
    for &i in &live_indices {
        let a = &episode.agents[i];
        let non_idle_ok = if params::ECO_REQUIRE_NON_IDLE_FOR_BIRTH {
            a.body.vel.length() > 0.2 || a.idle_steps < IDLENESS_THRESHOLD_STEPS
        } else { true };
        if a.repro_cooldown == 0 && a.offspring_count < ECO_MAX_OFFSPRING_PER_AGENT && non_idle_ok {
            eligible.push(i);
        }
    }

    // Attempt to find pairs among eligible agents. Only allow mating within the same kind (Herbivore/Carnivore).
    let cost = ECO_BIRTH_ENERGY_COST;
    let mut births: Vec<(usize, usize, crate::body::Body)> = Vec::new(); // (p1_idx, p2_idx, child_body)
    let mut used: std::collections::HashSet<usize> = std::collections::HashSet::new();
    eligible.sort_unstable();
    'pairing: for (ii, &i) in eligible.iter().enumerate() {
        if used.contains(&i) { continue; }
        let ai = &episode.agents[i];
        for &j in eligible.iter().skip(ii+1) {
            if used.contains(&j) { continue; }
            let aj = &episode.agents[j];
            // Kind check: must be the same ecological kind
            if ai.kind != aj.kind { continue; }
            // Energy cost feasibility
            let half = cost * 0.5; if ai.energy < half || aj.energy < half { continue; }
            // Require agents to be touching (wrap-aware shortest distance)
            let mut dx = ai.body.pos.x - aj.body.pos.x;
            if dx.abs() > WORLD_W * 0.5 {
                if dx > 0.0 { dx -= WORLD_W; } else { dx += WORLD_W; }
            }
            let mut dy = ai.body.pos.y - aj.body.pos.y;
            if dy.abs() > WORLD_H * 0.5 {
                if dy > 0.0 { dy -= WORLD_H; } else { dy += WORLD_H; }
            }
            let dist2 = dx*dx + dy*dy;
            let touch_r = ai.body.radius + aj.body.radius;
            if dist2 > touch_r * touch_r { continue; }

            // Child spawn near parents: midpoint with small jitter
            let mid = Vec2 { x: (ai.body.pos.x + aj.body.pos.x) * 0.5, y: (ai.body.pos.y + aj.body.pos.y) * 0.5 };
            let jitter = Vec2 { x: (rng.random::<f32>() - 0.5) * 3.0 * AGENT_RADIUS, y: (rng.random::<f32>() - 0.5) * 3.0 * AGENT_RADIUS };
            let mut pos = mid + jitter;
            pos.x = (pos.x % WORLD_W + WORLD_W) % WORLD_W; pos.y = (pos.y % WORLD_H + WORLD_H) % WORLD_H;
            births.push((i, j, crate::body::Body { pos, vel: Vec2::new(0.0, 0.0), radius: AGENT_COLLISION_RADIUS }));
            used.insert(i); used.insert(j);
            if live_indices.len() + births.len() >= ECO_MAX_POP { break 'pairing; }
        }
    }

    if births.is_empty() { return 0; }

    // Realize births: create child via crossover + mutation; debit both parents; set cooldowns; append agent + genome
    let mut realized = 0usize;
    for (i, j, body) in births {
        // Double-check alignment: parents indices map to genome indices
        if i >= population.len() || j >= population.len() { continue; }
        let p1 = &population[i];
        let p2 = &population[j];
        // Offspring are produced by crossover from parents and immediately mutated (per request)
        let mut child_g = neat::neat::crossover::crossover(p1, p2);
        child_g.mutate(_innov, _cfg);
        population.push(child_g);

        // Decide newborn kind: inherit if parents share kind, otherwise random choice
        let child_kind = {
            let ai_kind = episode.agents[i].kind;
            let aj_kind = episode.agents[j].kind;
            // Parents are required to be same kind; inherit directly
            if ai_kind == aj_kind { ai_kind } else { ai_kind }
        };
        // Append newborn agent aligned with last genome
        let birth_pos = body.pos;  // Save position before moving body
        episode.agents.push(Agent {
            id: AgentId(episode.agents.len()),
            kind: child_kind,
            body,
            theta: -std::f32::consts::FRAC_PI_2,
            energy: ECO_NEWBORN_ENERGY.min(MAX_ENERGY),
            health: ECO_NEWBORN_HEALTH,
            max_health: ECO_NEWBORN_HEALTH,
            invuln_steps: 0,
            alive_steps: 0,
            eaten: 0,
            consumed: false,
            kills: 0,
            predation_flash_steps: 0,
            dead_since: None,
            corpse_energy: 0.0,
            digest: std::collections::VecDeque::new(),
            last_food_mem: Vec2 { x: 0.0, y: 0.0 },
            last_danger_mem: Vec2 { x: 0.0, y: 0.0 },
            last_same_mem: Vec2 { x: 0.0, y: 0.0 },
            last_other_mem: Vec2 { x: 0.0, y: 0.0 },
            // For code paths that still read species_id, mirror kind: Herbivore=0, Carnivore=1
            species_id: match child_kind { crate::sim::AgentKind::Herbivore => 0, crate::sim::AgentKind::Carnivore => 1 },
            age_steps: 0,
            call_intensity: 0.0,
            heard_sectors: [0.0;3],
            repro_cooldown: ECO_BIRTH_COOLDOWN_STEPS,
            offspring_count: 0,
            attack_hits: 0,
            kills_caused: 0,
            idle_anchor: birth_pos,
            idle_steps: 0,
            total_idle_steps: 0,
            total_idle_penalty: 0.0,
            input_buf: vec![0.0; crate::params::INPUTS],
            energy_accum: 0.0,
            herding_units: 0.0,
            approach_food_units: 0.0,
            chase_other_units: 0.0,
            chase_same_units: 0.0,
        });
        // Extend comm fitness accumulator to match agents length
        episode.comm_fitness_accum.push(0.0);
        episode.births_this_episode += 1;
        match child_kind {
            crate::sim::AgentKind::Herbivore => { episode.births_herb += 1; }
            crate::sim::AgentKind::Carnivore => { episode.births_carn += 1; }
        }

        // Apply costs and cooldowns to parents; then push them apart to reduce clustering after birth
        {
            let (a_idx, b_idx) = if i < j { (i, j) } else { (j, i) };
            let (left, right) = episode.agents.split_at_mut(b_idx);
            let pa = &mut left[a_idx];
            let pb = &mut right[0];
            pa.energy = (pa.energy - cost * 0.5).max(0.0);
            pb.energy = (pb.energy - cost * 0.5).max(0.0);
            pa.offspring_count += 1;
            pb.offspring_count += 1;
            // Scale cooldown by current offspring count to space repeated births
            pa.repro_cooldown = ECO_BIRTH_COOLDOWN_STEPS + (pa.offspring_count * (ECO_BIRTH_COOLDOWN_STEPS / 2));
            pb.repro_cooldown = ECO_BIRTH_COOLDOWN_STEPS + (pb.offspring_count * (ECO_BIRTH_COOLDOWN_STEPS / 2));
            // Separation impulse to reduce clustering after birth
            let sep_ab = pa.body.pos - pb.body.pos;
            let len = sep_ab.length();
            if len > 1e-3 {
                let dir = sep_ab / len;
                pa.body.vel += dir * BIRTH_SEPARATION_IMPULSE;
                pb.body.vel -= dir * BIRTH_SEPARATION_IMPULSE;
            }
        }
    // Award unit reproduction credit to parents; scaled by fitness weights later
    if let Some(fit) = episode.comm_fitness_accum.get_mut(i) { *fit += 1.0; }
    if let Some(fit) = episode.comm_fitness_accum.get_mut(j) { *fit += 1.0; }
        realized += 1;
    }
    realized
}

// draw_text_clamped moved to ui_common::draw_text_clamped
