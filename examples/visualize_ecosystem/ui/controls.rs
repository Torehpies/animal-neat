use macroquad::prelude::*;

/// Result of processing keyboard input that main loop must act on (some actions require
/// calling functions that live in `visualize_ecosystem.rs`, e.g. `finalize_end_of_episode`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlsAction {
    None,
    /// Request to open the in-sim async menu (ESC pressed when not focused on an agent)
    OpenMenu,
    /// Request to finalize an ended episode (used when scoreboard is pending and user pressed 'C')
    FinalizeEndOfEpisode,
}

/// Handle non-async keyboard shortcuts. Returns an action for the caller to handle when
/// the action needs to call back into the main file (for example `finalize_end_of_episode`).
pub fn handle_keyboard(
    state: &mut crate::AppState,
    running: &mut bool,
    fast_mode: &mut bool,
    click_cooldown: &mut f32,
) -> ControlsAction {
    use crate::ui_graphs;
    // Basic toggles
    if is_key_pressed(KeyCode::P) { *running = !*running; }
    if is_key_pressed(KeyCode::F) { *fast_mode = !*fast_mode; }
    if is_key_pressed(KeyCode::R) {
        // Re-apply runtime configuration from the UI SimConfig so a keyboard reset follows
        // the user's chosen world/population/energy settings.
        let sc = &state.sim_config;
        crate::world::set_runtime_config(sc.world_width, sc.world_height, sc.max_food, sc.food_respawn_prob);
        // per-kind energy and population size setters live in params
        crate::params::set_runtime_energy_config_per_kind(
            sc.initial_energy_herb,
            sc.max_energy_herb,
            sc.energy_drain_per_step_herb,
            sc.initial_energy_carn,
            sc.max_energy_carn,
            sc.energy_drain_per_step_carn,
        );
        crate::params::set_runtime_population_size(sc.population_size);
        // If the user changed population size in the SimConfig, recreate the population
        // so the new episode uses the requested number of agents/genomes.
        if sc.population_size != state.population.len() {
            // Recreate population using existing innovation tracker
            let num_inputs = crate::params::INPUTS as u32;
            let num_outputs = crate::params::OUTPUTS as u32;
            state.population = neat::neat::genome::Genome::create_initial_population(
                sc.population_size,
                num_inputs,
                num_outputs,
                &mut state.innov,
            );
            // Respeciate for coloring/metadata
            state.speciator.get_species_mut().clear();
            state.speciator.speciate(&state.population);
            state.member_species = {
                let mut map = vec![0usize; state.population.len()];
                for (sidx, s) in state.speciator.get_species().iter().enumerate() {
                    for &m in &s.members { if m < state.population.len() { map[m] = sidx; } }
                }
                map
            };
        }
        let mut rng = ::rand::rng();
        state.episode = crate::sim::Episode::new(&mut rng, state.population.len());
    }
    if is_key_pressed(KeyCode::V) { state.show_cones = !state.show_cones; }
    if is_key_pressed(KeyCode::U) { state.show_unified_overlay = !state.show_unified_overlay; }
    if is_key_pressed(KeyCode::E) { state.show_energy_overlay = !state.show_energy_overlay; }

    // 'C' key: if scoreboard is open, request finalize; otherwise toggle collision radii
    if is_key_pressed(KeyCode::C) {
        if state.scoreboard_pending {
            return ControlsAction::FinalizeEndOfEpisode;
        } else {
            state.show_collision_radii = !state.show_collision_radii;
        }
    }

    if is_key_pressed(KeyCode::G) { state.show_grid = !state.show_grid; }
    if is_key_pressed(KeyCode::N) { state.show_best_network_panel = !state.show_best_network_panel; }
    if is_key_pressed(KeyCode::H) { state.show_controls = !state.show_controls; }
    if is_key_pressed(KeyCode::K) { state.color_by_species = !state.color_by_species; }

    // Graphs overlay toggle and tab navigation
    if is_key_pressed(KeyCode::Z) { state.show_graphs_overlay = !state.show_graphs_overlay; }
    if state.show_graphs_overlay {
        if is_key_pressed(KeyCode::Right) {
            state.graphs_tab = match state.graphs_tab {
                ui_graphs::GraphTab::Population => ui_graphs::GraphTab::Fitness,
                ui_graphs::GraphTab::Fitness => ui_graphs::GraphTab::BirthsDeaths,
                ui_graphs::GraphTab::BirthsDeaths => ui_graphs::GraphTab::Intelligence,
                ui_graphs::GraphTab::Intelligence => ui_graphs::GraphTab::Population,
            }
        }
        if is_key_pressed(KeyCode::Left) {
            state.graphs_tab = match state.graphs_tab {
                ui_graphs::GraphTab::Population => ui_graphs::GraphTab::Intelligence,
                ui_graphs::GraphTab::Fitness => ui_graphs::GraphTab::Population,
                ui_graphs::GraphTab::BirthsDeaths => ui_graphs::GraphTab::Fitness,
                ui_graphs::GraphTab::Intelligence => ui_graphs::GraphTab::BirthsDeaths,
            }
        }
        if is_key_pressed(KeyCode::Key1) { state.graphs_tab = ui_graphs::GraphTab::Population; }
        if is_key_pressed(KeyCode::Key2) { state.graphs_tab = ui_graphs::GraphTab::Fitness; }
        if is_key_pressed(KeyCode::Key3) { state.graphs_tab = ui_graphs::GraphTab::BirthsDeaths; }
        if is_key_pressed(KeyCode::Key4) { state.graphs_tab = ui_graphs::GraphTab::Intelligence; }
    }

    if is_key_pressed(KeyCode::X) { state.ultra_mode = !state.ultra_mode; }

    // Scoreboard toggle preference (T)
    if is_key_pressed(KeyCode::T) {
        if !state.scoreboard_pending {
            state.show_scoreboard_panel = !state.show_scoreboard_panel;
        } else {
            // Toggle preference only; keep scoreboard open until user continues
            state.show_scoreboard_panel = !state.show_scoreboard_panel;
        }
    }

    // Quick-save (S)
    if is_key_pressed(KeyCode::S) {
        let filename = format!("{}__quicksave.json", state.save_prefix);
        if let Err(e) = crate::snapshot::save_sim_snapshot(&filename, state.generation, &state.population, &state.innov, &state.episode, &state.member_species) {
            eprintln!("Failed to quick-save sim snapshot: {}", e);
            state.hud_toast = Some((format!("Save failed: {}", e), 3.5));
        } else {
            println!("Quick-saved sim snapshot to {}", filename);
            state.hud_toast = Some((format!("Saved: {}", filename), 2.5));
        }
    }

    // Load most-recent snapshot (L) - tries full sim snapshot then falls back to population-only
    if is_key_pressed(KeyCode::L) {
        let mut latest: Option<(std::path::PathBuf, std::time::SystemTime)> = None;
        if let Ok(entries) = std::fs::read_dir("snapshots") {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.extension().and_then(|s| s.to_str()) == Some("json") {
                    if let Ok(meta) = entry.metadata() {
                        if let Ok(mtime) = meta.modified() {
                            if latest.is_none() || mtime > latest.as_ref().unwrap().1 {
                                latest = Some((p, mtime));
                            }
                        }
                    }
                }
            }
        }
        if let Some((path, _)) = latest {
            // Try loading as full sim snapshot first
            match crate::snapshot::load_sim_snapshot(path.to_str().unwrap()) {
                Ok(snap) => {
                    state.population = snap.population;
                    state.generation = snap.generation;
                    state.innov = snap.innovation;
                    state.episode.food = snap.food.iter().map(|v| v.to_vec2()).collect();
                    // rebuild agents from snapshots (leave transient fields defaulted)
                    state.episode.agents.clear();
                    for (i, a_snap) in snap.agents.iter().enumerate() {
                        let body = crate::body::Body { pos: a_snap.body_pos.to_vec2(), vel: a_snap.body_vel.to_vec2(), radius: crate::params::AGENT_COLLISION_RADIUS };
                        state.episode.agents.push(crate::sim::Agent {
                            id: crate::sim::AgentId(i),
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
                    println!("Loaded sim snapshot from {}", path.display());
                    state.hud_toast = Some((format!("Loaded: {}", path.display()), 2.5));
                }
                Err(_) => {
                    // fallback: try legacy population-only snapshot
                    match neat::neat::io::load_population_snapshot(path.to_str().unwrap()) {
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
                            state.episode = crate::sim::Episode::new(&mut rng, state.population.len());
                            println!("Loaded population snapshot from {}", path.display());
                            state.hud_toast = Some((format!("Loaded population: {}", path.display()), 2.5));
                        }
                        Err(e) => {
                            eprintln!("Failed to load snapshot {}: {}", path.display(), e);
                            state.hud_toast = Some((format!("Load failed: {}", e), 3.5));
                        }
                    }
                }
            }
        } else {
            eprintln!("No snapshot files found in snapshots/");
            state.hud_toast = Some(("No snapshots found".to_string(), 2.5));
        }
    }

    // ESC handling: clear focus if focused, otherwise request menu open
    if is_key_pressed(KeyCode::Escape) {
        if state.focused_agent.is_some() {
            state.focused_agent = None;
            return ControlsAction::None;
        } else {
            return ControlsAction::OpenMenu;
        }
    }

    // Decrement click cooldown (kept here so the keyboard handler still manages it)
    if *click_cooldown > 0.0 {
        *click_cooldown -= get_frame_time();
        if *click_cooldown < 0.0 { *click_cooldown = 0.0; }
    }

    ControlsAction::None
}

/// Handle mouse-driven focus selection (left = pick agent; right = clear focus).
pub fn handle_mouse_buttons(state: &mut crate::AppState, mouse_world: Option<crate::Vec2>) {
    // Left click: pick nearest alive agent within some world-space radius
    if is_mouse_button_pressed(MouseButton::Left) {
        if let Some(mw) = mouse_world {
            let pick_r = crate::params::AGENT_COLLISION_RADIUS * 3.5;
            let mut best: Option<(usize, f32)> = None;
            for (i, a) in state.episode.agents.iter().enumerate() {
                if a.energy <= 0.0 && a.health <= crate::params::DEATH_HEALTH_THRESHOLD { continue; }
                let dx = a.body.pos.x - mw.x; let dy = a.body.pos.y - mw.y; let d2 = dx*dx + dy*dy;
                if d2 <= pick_r * pick_r {
                    if let Some((_, bd2)) = best { if d2 < bd2 { best = Some((i, d2)); } } else { best = Some((i, d2)); }
                }
            }
            state.focused_agent = best.map(|(i, _)| i);
        }
    }
    // Right click clears focus
    if is_mouse_button_pressed(MouseButton::Right) {
        state.focused_agent = None;
    }
}
