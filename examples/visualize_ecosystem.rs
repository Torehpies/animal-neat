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
#[path = "visualize_ecosystem/eco_evolution.rs"]
mod eco_evolution;
use eco_evolution::*;
use crate::sim::{Agent, AgentId};
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


