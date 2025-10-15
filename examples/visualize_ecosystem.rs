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
    io,
    speciator::Speciator,
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
#[path = "visualize_ecosystem/ui/menu.rs"]
mod ui_menu;
#[path = "visualize_ecosystem/ui/main_menu.rs"]
mod ui_main_menu;
#[path = "visualize_ecosystem/elements/body.rs"]
mod body;
#[path = "visualize_ecosystem/sim/mod.rs"]
mod sim;
#[path = "visualize_ecosystem/snapshot.rs"]
mod snapshot;
#[path = "visualize_ecosystem/ui/load_picker.rs"]
mod ui_load_picker;
#[path = "visualize_ecosystem/ui/save_picker.rs"]
mod ui_save_picker;
use sim::{Episode, Agent, AgentId};
use ::rand::Rng;
use params::*;
use sim::eval_population_single_episode;
use ui_menu::SimConfig;


// Episode methods are defined in sim::episode

struct AppState {
    population: Vec<Genome>,
    innov: InnovationTracker,
    speciator: Speciator,
    cfg: EvolutionConfig,
    generation: usize,
    last_best: f32,
    last_avg: f32,
    episode: Episode,
    show_cones: bool,
    member_species: Vec<usize>,
    // Best of last evaluated generation (for HUD panel)
    last_best_generation: usize,
    last_best_genome: Option<Genome>,
    // Debug overlays
    show_unified_overlay: bool,
    show_energy_overlay: bool,
    show_collision_radii: bool,
    show_grid: bool,
    // HUD/network & focus controls
    show_best_network_panel: bool,
    show_live_network: bool,
    focused_agent: Option<usize>,
    // Eco mode helpers
    eco_episode_counter: usize,
    show_controls: bool,
    color_by_species: bool,
    // Runtime config
    #[allow(dead_code)]
    pub sim_config: SimConfig,
    // Per-session save prefix (unique per new simulation)
    pub save_prefix: String,
}

impl AppState {
    fn new(sim_config: SimConfig) -> Self {
        let pop_size = sim_config.population_size;
        let num_inputs = INPUTS as u32;
        let num_outputs = OUTPUTS as u32;
        let mut rng = ::rand::rng();
        let mut innov = InnovationTracker::new();
        // Speciation target and adapt rate are now configurable via params
        let mut speciator = Speciator::new(1.0).with_target(SPECIES_TARGET, SPECIES_ADAPT_RATE);
        let cfg = EvolutionConfig { compatibility_threshold: 2.0, ..Default::default() };
        let population = Genome::create_initial_population(pop_size, num_inputs, num_outputs, &mut innov);
        // initial speciation for coloring
        speciator.speciate(&population);
        let member_species = {
            let mut map = vec![0usize; population.len()];
            for (sidx, s) in speciator.get_species().iter().enumerate() {
                for &m in &s.members { if m < population.len() { map[m] = sidx; } }
            }
            map
        };
    let episode = Episode::new(&mut rng, pop_size, &member_species);
        // Create a unique save prefix per simulation using system time
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
        let save_prefix = format!("snapshots/sim_{}_{:09}", now.as_secs(), now.subsec_nanos());
        Self {
            population,
            innov,
            speciator,
            cfg,
            generation: 0,
            last_best: f32::NEG_INFINITY,
            last_avg: 0.0,
            episode,
            show_cones: true,
            member_species,
            last_best_generation: 0,
            last_best_genome: None,
            show_unified_overlay: false,
            show_energy_overlay: true,
            show_collision_radii: false,
            show_grid: false,
            show_best_network_panel: true,
            show_live_network: false,
            focused_agent: None,
            eco_episode_counter: 0,
            show_controls: true,
            color_by_species: false,
            sim_config,
            save_prefix,
        }
    }

    fn eval_population(&mut self) -> Vec<f32> {
        // Multi-episode averaging with exploration reward and avoidance penalty
        let mut acc = vec![0.0f32; self.population.len()];
        for _ in 0..EPISODES_PER_GEN {
            let scores = eval_population_single_episode(&self.population);
            for (i, s) in scores.iter().enumerate() { acc[i] += *s; }
        }
        for v in &mut acc { *v /= EPISODES_PER_GEN as f32; }
        let best = acc.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        let avg = acc.iter().sum::<f32>() / acc.len() as f32;
        self.last_best = best;
        self.last_avg = avg;
        acc
    }

    fn evolve_one_generation(&mut self) {
        let fitness_scores = self.eval_population();
        // Update best-ever before population is replaced
    if let Some((best_idx, _)) = fitness_scores
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
        {
            // Track best of last evaluated generation for HUD graph
            self.last_best_generation = self.generation;
            if best_idx < self.population.len() {
                self.last_best_genome = Some(self.population[best_idx].clone());
            } else {
                self.last_best_genome = None;
            }
            // (Best-ever tracking removed for simplification)
        }
        // Evolve population
        self.population = evolution::evolution(
            std::mem::take(&mut self.population),
            fitness_scores,
            &mut self.speciator,
            &mut self.innov,
            &self.cfg,
        );
        // IMPORTANT: speciate BEFORE creating the next episode so agents get correct species IDs
        self.speciator.speciate(&self.population);
        self.member_species = {
            let mut map = vec![0usize; self.population.len()];
            for (sidx, s) in self.speciator.get_species().iter().enumerate() {
                for &m in &s.members { if m < self.population.len() { map[m] = sidx; } }
            }
            map
        };
        self.generation += 1;
        let mut rng = ::rand::rng();
        self.episode = Episode::new(&mut rng, self.population.len(), &self.member_species);
        // Clear focused agent because indices now refer to new episode
        self.focused_agent = None;
        // Optional periodic snapshotting after evolution completes this generation
        if SNAPSHOT_INTERVAL > 0 && self.generation % SNAPSHOT_INTERVAL == 0 {
            let filename = format!("{}__gen{:0>6}.json", self.save_prefix, self.generation);
            if let Err(e) = snapshot::save_sim_snapshot(&filename, self.generation, &self.population, &self.innov, &self.episode, &self.member_species) {
                eprintln!("Auto-snapshot failed: {e}");
            } else {
                println!("Auto-saved sim snapshot to {filename}");
            }
        }
    }
}

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
    'main_loop: loop {
        // Main menu runner (in separate module) – returns when user chooses to create a sim, load one, or exits
        let menu_res = ui_main_menu::run_main_menu().await;
        let (sim_config, loaded_snapshot_path) = match menu_res {
            ui_main_menu::MenuResult::New(cfg) => (cfg, None),
            ui_main_menu::MenuResult::Exit => return,
            ui_main_menu::MenuResult::Load(path) => (ui_menu::SimConfig::default(), Some(path)),
        };
        
        // Apply configuration to global params (via world module)
        world::set_runtime_config(sim_config.world_width, sim_config.world_height, sim_config.max_food, sim_config.food_respawn_prob);
        params::set_runtime_energy_config(sim_config.initial_energy, sim_config.max_energy, sim_config.energy_drain_per_step);
        params::set_runtime_population_size(sim_config.population_size);
        
        let mut state = AppState::new(sim_config);
        // If the main menu requested to load a snapshot, apply it now
        if let Some(path) = loaded_snapshot_path {
            if let Ok(snap) = snapshot::load_sim_snapshot(&path) {
                state.population = snap.population;
                state.generation = snap.generation;
                state.innov = snap.innovation;
                state.episode.food = snap.food.iter().map(|v| v.to_vec2()).collect();
                state.episode.agents.clear();
                for (i, a_snap) in snap.agents.iter().enumerate() {
                    let body = crate::body::Body { pos: a_snap.body_pos.to_vec2(), vel: a_snap.body_vel.to_vec2(), radius: AGENT_RADIUS };
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
        // In-simulation modal menu state
        let mut sim_menu_open: bool = false;
        let mut prev_running_state: bool = running;
        // Click cooldown (seconds) to avoid double/triple activations from fast clicks
        let mut click_cooldown: f32 = 0.0;
        let mut normal_step_timer = 0.0f32;          // accumulates frame time for normal stepping
        let normal_step_interval = 0.05f32;           // seconds per simulation step in normal mode
        let fast_steps_per_frame: usize = 500;       // simulation steps per frame in fast mode

        loop {
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
            if is_key_pressed(KeyCode::P) { running = !running; }
            if is_key_pressed(KeyCode::F) { fast_mode = !fast_mode; }
            if is_key_pressed(KeyCode::R) { let mut rng = ::rand::rng(); state.episode = Episode::new(&mut rng, state.population.len(), &state.member_species); }
        if is_key_pressed(KeyCode::V) { state.show_cones = !state.show_cones; }
            if is_key_pressed(KeyCode::U) { state.show_unified_overlay = !state.show_unified_overlay; }
            if is_key_pressed(KeyCode::E) { state.show_energy_overlay = !state.show_energy_overlay; }
        if is_key_pressed(KeyCode::C) { state.show_collision_radii = !state.show_collision_radii; }
        if is_key_pressed(KeyCode::G) { state.show_grid = !state.show_grid; }
        if is_key_pressed(KeyCode::N) { state.show_best_network_panel = !state.show_best_network_panel; }
        if is_key_pressed(KeyCode::M) { state.show_live_network = !state.show_live_network; }
        if is_key_pressed(KeyCode::H) { state.show_controls = !state.show_controls; }
        if is_key_pressed(KeyCode::K) { state.color_by_species = !state.color_by_species; }
        // Save snapshot (S) and Load most-recent snapshot (O)
        if is_key_pressed(KeyCode::S) {
            // Quick-save (overwrite) per-session file
            let filename = format!("{}__quicksave.json", state.save_prefix);
            if let Err(e) = snapshot::save_sim_snapshot(&filename, state.generation, &state.population, &state.innov, &state.episode, &state.member_species) {
                eprintln!("Failed to quick-save sim snapshot: {}", e);
            } else {
                println!("Quick-saved sim snapshot to {}", filename);
            }
        }
        if is_key_pressed(KeyCode::O) {
            // Find the most-recent *.json file in snapshots/ and attempt to load it
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
                match snapshot::load_sim_snapshot(path.to_str().unwrap()) {
                    Ok(snap) => {
                        state.population = snap.population;
                        state.generation = snap.generation;
                        state.innov = snap.innovation;
                        // restore world/episode state
                        state.episode.food = snap.food.iter().map(|v| v.to_vec2()).collect();
                        // rebuild agents from snapshots (leave transient fields defaulted)
                        state.episode.agents.clear();
                        for (i, a_snap) in snap.agents.iter().enumerate() {
                            let body = crate::body::Body { pos: a_snap.body_pos.to_vec2(), vel: a_snap.body_vel.to_vec2(), radius: AGENT_RADIUS };
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
                            });
                        }
                        state.episode.steps = snap.episode_steps;
                        // rebuild speciator and member_species mapping
                        state.speciator.get_species_mut().clear();
                        state.speciator.speciate(&state.population);
                        state.member_species = snap.member_species;
                        println!("Loaded sim snapshot from {}", path.display());
                    }
                    Err(_) => {
                        // fallback: try legacy population-only snapshot
                        match io::load_population_snapshot(path.to_str().unwrap()) {
                            Ok(snap) => {
                                state.population = snap.population;
                                state.generation = snap.generation;
                                state.innov = snap.innovation;
                                // Rebuild speciator and member_species mapping
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
                                state.episode = Episode::new(&mut rng, state.population.len(), &state.member_species);
                                println!("Loaded population snapshot from {}", path.display());
                            }
                            Err(e) => eprintln!("Failed to load snapshot {}: {}", path.display(), e),
                        }
                    }
                }
            } else {
                eprintln!("No snapshot files found in snapshots/");
            }
        }
        
        // ESC: if focused on an agent, clear focus; otherwise open in-sim menu overlay
        if is_key_pressed(KeyCode::Escape) {
            if state.focused_agent.is_some() {
                state.focused_agent = None;
            } else {
                // Open or close the in-simulation modal menu instead of going straight back to main menu
                sim_menu_open = !sim_menu_open;
                if sim_menu_open {
                    // pause simulation while menu is open
                    prev_running_state = running;
                    running = false;
                } else {
                    // restore running state when closing menu (unless explicitly resumed)
                    running = prev_running_state;
                }
            }
        }
        // Removed per-row overlay toggles (1..4). Unified overlay is controlled via 'U'.
        // Removed: [S] save snapshot and [B] easy birth debug toggle

        if running {
            let mut rng = ::rand::rng();
            if fast_mode {
                // Run many simulation steps per frame until the episode finishes, then evolve
                for _ in 0..fast_steps_per_frame {
                    if state.episode.is_finished() {
                        if ECO_CONTINUOUS {
                            // In eco mode: cull by fitness back to POPULATION_SIZE, then reseed next episode
                            state.eco_episode_counter += 1;
                            eco_cull_population_by_fitness(&mut state);
                            state.episode = Episode::new(&mut rng, state.population.len(), &state.member_species);
                            break;
                        } else {
                            state.evolve_one_generation();
                            state.episode = Episode::new(&mut rng, state.population.len(), &state.member_species);
                            break;
                        }
                    }
                    state.episode.step(&state.population, &mut rng);
                    if ECO_CONTINUOUS {
                        // Reproduction pass: try to spawn offspring for eligible parents
                        spawn_offspring_if_needed(
                            &mut state.population,
                            &mut state.episode,
                            &mut state.innov,
                            &state.cfg,
                            &mut rng,
                            &mut state.speciator,
                            &mut state.member_species,
                        );
                    }
                }
                // If it finished exactly on the last step, evolve now
                if state.episode.is_finished() {
                    if ECO_CONTINUOUS {
                        state.eco_episode_counter += 1;
                        eco_cull_population_by_fitness(&mut state);
                        state.episode = Episode::new(&mut rng, state.population.len(), &state.member_species);
                    } else {
                        state.evolve_one_generation();
                        state.episode = Episode::new(&mut rng, state.population.len(), &state.member_species);
                    }
                }
            } else {
                // Normal mode: advance one simulation step per second
                normal_step_timer += get_frame_time();
                if normal_step_timer >= normal_step_interval {
                    normal_step_timer -= normal_step_interval;
                    if state.episode.is_finished() {
                        if ECO_CONTINUOUS {
                            state.eco_episode_counter += 1;
                            eco_cull_population_by_fitness(&mut state);
                            state.episode = Episode::new(&mut rng, state.population.len(), &state.member_species);
                        } else {
                            state.evolve_one_generation();
                            state.episode = Episode::new(&mut rng, state.population.len(), &state.member_species);
                        }
                    } else {
                        state.episode.step(&state.population, &mut rng);
                        if ECO_CONTINUOUS {
                            spawn_offspring_if_needed(
                                &mut state.population,
                                &mut state.episode,
                                &mut state.innov,
                                &state.cfg,
                                &mut rng,
                                &mut state.speciator,
                                &mut state.member_species,
                            );
                        }
                        if state.episode.is_finished() {
                            if ECO_CONTINUOUS {
                                state.eco_episode_counter += 1;
                                eco_cull_population_by_fitness(&mut state);
                                state.episode = Episode::new(&mut rng, state.population.len(), &state.member_species);
                            } else {
                                state.evolve_one_generation();
                                state.episode = Episode::new(&mut rng, state.population.len(), &state.member_species);
                            }
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

    // Agent focus selection on left click
    if is_mouse_button_pressed(MouseButton::Left) {
        if let Some(mw) = mouse_world {
            // Find nearest alive agent within a pick radius in world units
            let pick_r = AGENT_RADIUS * 3.5; // generous
            let mut best: Option<(usize, f32)> = None;
            for (i, a) in state.episode.agents.iter().enumerate() {
                if a.energy <= 0.0 && a.health <= DEATH_HEALTH_THRESHOLD { continue; }
                let dx = a.body.pos.x - mw.x; let dy = a.body.pos.y - mw.y; let d2 = dx*dx + dy*dy;
                if d2 <= pick_r * pick_r {
                    if let Some((_, bd2)) = best { if d2 < bd2 { best = Some((i, d2)); } } else { best = Some((i, d2)); }
                }
            }
            state.focused_agent = best.map(|(i, _)| i);
        }
    }
    // Right click clears focus (unfocus)
    if is_mouse_button_pressed(MouseButton::Right) {
        state.focused_agent = None;
    }
    // (mouse_world already defined above)

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
    );
    ui_hud::draw_hud(hud_area, &state, running, fast_mode, &state.member_species);
        // Decrement click cooldown
        if click_cooldown > 0.0 {
            click_cooldown -= get_frame_time();
            if click_cooldown < 0.0 { click_cooldown = 0.0; }
        }

        // If the simulation menu overlay is open, draw it on top and handle its input.
        if sim_menu_open {
            // Modal panel
            let panel_w = 420.0;
            let panel_h = 240.0;
            let cx = screen_width() * 0.5;
            let cy = screen_height() * 0.5;
            let panel_x = cx - panel_w * 0.5;
            let panel_y = cy - panel_h * 0.5;

            draw_rectangle(panel_x, panel_y, panel_w, panel_h, Color::new(0.06, 0.06, 0.08, 0.95));
            draw_rectangle_lines(panel_x, panel_y, panel_w, panel_h, 2.0, GRAY);
            let title = "Simulation Menu";
            let tw = measure_text(title, None, 28, 1.0).width;
            draw_text(title, cx - tw * 0.5, panel_y + 36.0, 28.0, WHITE);

            let btn_w = 140.0;
            let btn_h = 40.0;
            let gap = 18.0;
            let left_x = panel_x + 32.0;
            let mut by = panel_y + 72.0;

            // Resume button
            let resume_x = left_x;
            let resume_y = by;
            let (mx, my) = mouse_position();
            let hovering_resume = mx >= resume_x && mx <= resume_x + btn_w && my >= resume_y && my <= resume_y + btn_h;
            let resume_col = if hovering_resume { Color::new(0.25, 0.7, 0.25, 1.0) } else { Color::new(0.18, 0.5, 0.18, 1.0) };
            draw_rectangle(resume_x, resume_y, btn_w, btn_h, resume_col);
            draw_rectangle_lines(resume_x, resume_y, btn_w, btn_h, 2.0, WHITE);
            let txt = "Resume";
            let tw = measure_text(txt, None, 20, 1.0).width;
            draw_text(txt, resume_x + (btn_w - tw) / 2.0, resume_y + 26.0, 20.0, WHITE);

            // Save button (open save dialog)
            let save_x = resume_x + btn_w + gap;
            let save_y = by;
            let hovering_save = mx >= save_x && mx <= save_x + btn_w && my >= save_y && my <= save_y + btn_h;
            let save_col = if hovering_save { Color::new(0.25, 0.6, 0.9, 1.0) } else { Color::new(0.15, 0.45, 0.75, 1.0) };
            draw_rectangle(save_x, save_y, btn_w, btn_h, save_col);
            draw_rectangle_lines(save_x, save_y, btn_w, btn_h, 2.0, WHITE);
            let txt = "Save...";
            let tw = measure_text(txt, None, 20, 1.0).width;
            draw_text(txt, save_x + (btn_w - tw) / 2.0, save_y + 26.0, 20.0, WHITE);

            // Load button (open load picker)
            let load_x = save_x + btn_w + gap;
            let load_y = by;
            let hovering_load = mx >= load_x && mx <= load_x + btn_w && my >= load_y && my <= load_y + btn_h;
            let load_col = if hovering_load { Color::new(0.9, 0.6, 0.25, 1.0) } else { Color::new(0.7, 0.45, 0.12, 1.0) };
            draw_rectangle(load_x, load_y, btn_w, btn_h, load_col);
            draw_rectangle_lines(load_x, load_y, btn_w, btn_h, 2.0, WHITE);
            let txt = "Load...";
            let tw = measure_text(txt, None, 20, 1.0).width;
            draw_text(txt, load_x + (btn_w - tw) / 2.0, load_y + 26.0, 20.0, WHITE);

            by += btn_h + 18.0;

            // Back to Main Menu button
            let back_x = left_x;
            let back_y = by;
            let back_w = panel_w - 64.0;
            let hovering_back = mx >= back_x && mx <= back_x + back_w && my >= back_y && my <= back_y + btn_h;
            let back_col = if hovering_back { Color::new(0.8, 0.25, 0.25, 1.0) } else { Color::new(0.6, 0.18, 0.18, 1.0) };
            draw_rectangle(back_x, back_y, back_w, btn_h, back_col);
            draw_rectangle_lines(back_x, back_y, back_w, btn_h, 2.0, WHITE);
            let txt = "Back to Main Menu";
            let tw = measure_text(txt, None, 20, 1.0).width;
            draw_text(txt, back_x + (back_w - tw) / 2.0, back_y + 26.0, 20.0, WHITE);

            // Handle mouse clicks for modal buttons (consume while modal open)
            if is_mouse_button_pressed(MouseButton::Left) {
                // Only accept clicks when cooldown expired
                if click_cooldown <= 0.0 {
                    if hovering_resume {
                        // Close menu and resume
                        sim_menu_open = false;
                        running = true;
                        click_cooldown = 0.25;
                    } else if hovering_save {
                        // Close menu, wait a short cooldown, then open save picker
                        sim_menu_open = false;
                        click_cooldown = 0.20;
                        while click_cooldown > 0.0 {
                            let dt = get_frame_time();
                            click_cooldown -= dt;
                            next_frame().await;
                        }
                        if let Some(path) = ui_save_picker::pick_save(None).await {
                            match snapshot::save_sim_snapshot(&path, state.generation, &state.population, &state.innov, &state.episode, &state.member_species) {
                                Ok(_) => println!("Saved sim snapshot to {}", path),
                                Err(e) => eprintln!("Failed to save sim snapshot {}: {}", path, e),
                            }
                        }
                        // re-open menu for continued interaction
                        sim_menu_open = true;
                        click_cooldown = 0.25;
                    } else if hovering_load {
                        // Close menu, wait a short cooldown, then open load picker
                        sim_menu_open = false;
                        click_cooldown = 0.20;
                        while click_cooldown > 0.0 {
                            let dt = get_frame_time();
                            click_cooldown -= dt;
                            next_frame().await;
                        }
                        if let Some(path) = ui_load_picker::pick_snapshot().await {
                            // Attempt to load similar to quick-load logic
                            if let Ok(snap) = snapshot::load_sim_snapshot(&path) {
                                state.population = snap.population;
                                state.generation = snap.generation;
                                state.innov = snap.innovation;
                                state.episode.food = snap.food.iter().map(|v| v.to_vec2()).collect();
                                state.episode.agents.clear();
                                for (i, a_snap) in snap.agents.iter().enumerate() {
                                    let body = crate::body::Body { pos: a_snap.body_pos.to_vec2(), vel: a_snap.body_vel.to_vec2(), radius: AGENT_RADIUS };
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
                                    });
                                }
                                state.episode.steps = snap.episode_steps;
                                state.speciator.get_species_mut().clear();
                                state.speciator.speciate(&state.population);
                                state.member_species = snap.member_species;
                                println!("Loaded sim snapshot from {}", path);
                            } else {
                                // fallback to population-only snapshot
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
                                        state.episode = Episode::new(&mut rng, state.population.len(), &state.member_species);
                                        println!("Loaded population snapshot from {}", path);
                                    }
                                    Err(e) => eprintln!("Failed to load snapshot {}: {}", path, e),
                                }
                            }
                        }
                        // restore paused state to showing the menu so the user can continue interacting
                        sim_menu_open = true;
                    } else if hovering_back {
                        // Return to main menu
                        // return to main menu by breaking to the outer 'main_loop
                        break 'main_loop;
                    }
                }
            }
        }

        next_frame().await;
    } // end 'sim_loop
    } // end 'main_loop
}

fn eco_cull_population_by_fitness(state: &mut AppState) {
    // Evaluate current population with the same single-episode fitness used for evolution
    let mut scores = eval_population_single_episode(&state.population);
    // Augment scores with reproduction reward from the just-finished live episode
    // Parents get a bonus per successful birth this episode.
    for (i, a) in state.episode.agents.iter().enumerate() {
        if i < scores.len() {
            scores[i] += (a.offspring_count as f32) * REPRO_BIRTH_FITNESS_PARENT;
        }
    }
    // Build indices sorted by fitness desc
    let mut all_idxs: Vec<usize> = (0..state.population.len()).collect();
    all_idxs.sort_by(|&a, &b| scores[b]
        .partial_cmp(&scores[a])
        .unwrap_or(std::cmp::Ordering::Equal));

    // 1) Build by-species lists sorted by within-species fitness
    use std::collections::HashMap;
    let mut by_species: HashMap<usize, Vec<usize>> = HashMap::new();
    for (idx, sid) in state.member_species.iter().enumerate() { by_species.entry(*sid).or_default().push(idx); }
    for v in by_species.values_mut() {
        v.sort_by(|&a, &b| scores[b]
            .partial_cmp(&scores[a])
            .unwrap_or(std::cmp::Ordering::Equal));
    }

    let mut selected: Vec<usize> = Vec::new();

    if EQUAL_ALLOC_ENABLED && !by_species.is_empty() {
        // Rank species by their best member's fitness
        let mut species_best: Vec<(usize, f32)> = by_species
            .iter()
            .map(|(sid, members)| {
                let best_score = members
                    .iter()
                    .copied()
                    .map(|i| scores[i])
                    .fold(f32::NEG_INFINITY, f32::max);
                (*sid, best_score)
            })
            .collect();
        species_best.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let k = EQUAL_ALLOC_TOP_K.max(1).min(species_best.len());
    let chosen_species: Vec<usize> = species_best.into_iter().take(k).map(|(sid, _)| sid).collect();

        // Compute base quota and distribute remainder to top species
    let pop_cap = params::get_population_size();
    let base = pop_cap / k;
    let remainder = pop_cap % k;

        // Take from each chosen species up to its quota, or all available if fewer
        for (rank, sid) in chosen_species.iter().enumerate() {
            if selected.len() >= pop_cap { break; }
            let quota = base + if rank < remainder { 1 } else { 0 };
            if let Some(members) = by_species.get(sid) {
                let take = members.len().min(quota);
                selected.extend_from_slice(&members[..take]);
            }
        }
        // If underfilled (some species didn't have enough members), fill by global fitness
        if selected.len() < pop_cap {
            let mut seen: std::collections::HashSet<usize> = selected.iter().copied().collect();
            for i in &all_idxs {
                if selected.len() >= pop_cap { break; }
                if !seen.contains(i) { selected.push(*i); seen.insert(*i); }
            }
        }
    } else {
        // Fallback: Keep up to ECO_CULL_MIN_PER_SPECIES per species by best fitness within that species
        if ECO_CULL_MIN_PER_SPECIES > 0 {
            for v in by_species.values() {
                let take = v.len().min(ECO_CULL_MIN_PER_SPECIES);
                selected.extend_from_slice(&v[..take]);
            }
        }

        // 2) Fill remaining slots by global fitness order, skipping already selected
        let mut seen: std::collections::HashSet<usize> = selected.iter().copied().collect();
        for i in &all_idxs {
            if selected.len() >= params::get_population_size() { break; }
            if !seen.contains(i) { selected.push(*i); seen.insert(*i); }
        }

        // If we still exceed population size (e.g., many species × min), trim globally
        if selected.len() > params::get_population_size() {
            selected.sort_by(|&a, &b| scores[b]
                .partial_cmp(&scores[a])
                .unwrap_or(std::cmp::Ordering::Equal));
            selected.truncate(params::get_population_size());
        }
    }

    // Rebuild population vector in selected order
    let mut new_pop = Vec::with_capacity(selected.len());
    for &i in &selected { new_pop.push(state.population[i].clone()); }
    state.population = new_pop;

    // After culling, perform mutation pass on the survivor population (post-episode),
    // keeping with standard NEAT where mutation happens between generations/episodes.
    // Apply mutation to all but optionally the top elite (could be added later if desired).
    for g in state.population.iter_mut() {
        g.mutate(&mut state.innov, &state.cfg);
    }
    // Update best/avg stats for HUD
    if !scores.is_empty() && !state.population.is_empty() {
        // Recompute best/avg over kept indices
        let best_score = selected.iter().copied().map(|i| scores[i]).fold(f32::NEG_INFINITY, f32::max);
        let avg_score = selected.iter().copied().map(|i| scores[i]).sum::<f32>() / (selected.len() as f32);
        state.last_best = best_score;
        state.last_avg = avg_score;
        state.last_best_generation = state.eco_episode_counter; // repurpose as last eval eco-episode id
        // Snapshot the best genome for the network panel
        // Find which kept index had best score and clone its genome
        if let Some((&best_idx, _)) = selected.iter().zip(selected.iter().map(|&i| scores[i])).max_by(|a,b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal)) {
            let pos = selected.iter().position(|&x| x == best_idx).unwrap_or(0);
            state.last_best_genome = state.population.get(pos).cloned();
        } else {
            state.last_best_genome = state.population.get(0).cloned();
        }
    }
        // Important: clear existing species to avoid stale representative indices
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

fn spawn_offspring_if_needed<R: Rng>(
    population: &mut Vec<Genome>,
    episode: &mut Episode,
    _innov: &mut InnovationTracker,
    _cfg: &EvolutionConfig,
    rng: &mut R,
    speciator: &mut Speciator,
    member_species: &mut Vec<usize>,
) {
    // Count live agents and skip if at cap
    let mut live_indices: Vec<usize> = Vec::new();
    for (i, a) in episode.agents.iter().enumerate() {
        if a.energy > 0.0 && a.health > DEATH_HEALTH_THRESHOLD && !a.consumed { live_indices.push(i); }
    }
    if live_indices.len() >= ECO_MAX_POP { return; }

    // First, tick down cooldowns for all alive agents
    for i in &live_indices {
        let a = &mut episode.agents[*i];
        if a.repro_cooldown > 0 { a.repro_cooldown -= 1; }
    }

    // Gather eligible parents by species (meets energy, cooldown, offspring cap)
    let threshold = ECO_BIRTH_ENERGY_THRESHOLD;
    let mut by_species: std::collections::HashMap<usize, Vec<usize>> = std::collections::HashMap::new();
    for &i in &live_indices {
        let a = &episode.agents[i];
        if a.repro_cooldown == 0 && a.offspring_count < ECO_MAX_OFFSPRING_PER_AGENT && a.energy >= threshold {
            by_species.entry(a.species_id).or_default().push(i);
        }
    }

    // Attempt to find nearby pairs within species and spawn one child per found pair this step
    let cost = ECO_BIRTH_ENERGY_COST;
    let mut births: Vec<(usize, usize, crate::body::Body, usize)> = Vec::new(); // (p1_idx, p2_idx, child_body, species_id)
    for (sid, indices) in by_species.into_iter() {
        // Simple n^2 pairing; early exit when near pop cap
        let mut used: std::collections::HashSet<usize> = std::collections::HashSet::new();
        'outer: for (ii, &i) in indices.iter().enumerate() {
            if used.contains(&i) { continue; }
            let ai = &episode.agents[i];
            for &j in indices.iter().skip(ii+1) {
                if used.contains(&j) { continue; }
                let aj = &episode.agents[j];
                // Distance check
                let dx = ai.body.pos.x - aj.body.pos.x; let dy = ai.body.pos.y - aj.body.pos.y;
                if dx*dx + dy*dy <= ECO_MATE_RADIUS*ECO_MATE_RADIUS {
                    // Both will pay half cost; ensure after payment they stay >= 0 energy
                    let half = cost * 0.5;
                    if ai.energy >= half && aj.energy >= half {
                        // Child spawn mid-point with small jitter
                        let mid = Vec2 { x: (ai.body.pos.x + aj.body.pos.x) * 0.5, y: (ai.body.pos.y + aj.body.pos.y) * 0.5 };
                        let jitter = Vec2 { x: (rng.random::<f32>() - 0.5) * 3.0 * AGENT_RADIUS, y: (rng.random::<f32>() - 0.5) * 3.0 * AGENT_RADIUS };
                        let mut pos = mid + jitter;
                        pos.x = (pos.x % WORLD_W + WORLD_W) % WORLD_W; pos.y = (pos.y % WORLD_H + WORLD_H) % WORLD_H;
                        births.push((i, j, crate::body::Body { pos, vel: Vec2::new(0.0, 0.0), radius: AGENT_RADIUS }, sid));
                        used.insert(i); used.insert(j);
                        if live_indices.len() + births.len() >= ECO_MAX_POP { break 'outer; }
                    }
                }
            }
        }
    }

    if births.is_empty() { return; }

    // Realize births: create child via crossover + mutation; debit both parents; set cooldowns; append agent + genome
    for (i, j, body, sid) in births {
        // Double-check alignment: parents indices map to genome indices
        if i >= population.len() || j >= population.len() { continue; }
        let p1 = &population[i];
        let p2 = &population[j];
    // Offspring are produced by crossover only; no mutation here (mutation happens after episode)
    let child_g = neat::neat::crossover::crossover(p1, p2);
        population.push(child_g);

        // Update species map using existing speciator (child species determined after speciation)
        speciator.speciate(&population);
        member_species.clear();
        member_species.resize(population.len(), 0);
        for (sidx, s) in speciator.get_species().iter().enumerate() {
            for &m in &s.members { if m < population.len() { member_species[m] = sidx; } }
        }
        // Append newborn agent aligned with last genome
        let birth_pos = body.pos;  // Save position before moving body
        episode.agents.push(Agent {
            id: AgentId(episode.agents.len()),
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
            // For live-episode behavior (predation/mating), tag newborn with the parents' species id
            // to avoid immediate conspecific misclassification due to structural differences.
            species_id: sid,
            age_steps: 0,
            call_intensity: 0.0,
            heard_sectors: [0.0;3],
            repro_cooldown: ECO_BIRTH_COOLDOWN_STEPS,
            offspring_count: 0,
            attack_hits: 0,
            kills_caused: 0,
            idle_anchor: birth_pos,
            idle_steps: 0,
            total_idle_penalty: 0.0,
        });
        // Extend comm fitness accumulator to match agents length
        episode.comm_fitness_accum.push(0.0);
        episode.births_this_episode += 1;

        // Apply costs and cooldowns to parents; award reproduction fitness bonus to parents
        if let Some(pa) = episode.agents.get_mut(i) {
            pa.energy = (pa.energy - cost * 0.5).max(0.0);
            pa.repro_cooldown = ECO_BIRTH_COOLDOWN_STEPS;
            pa.offspring_count += 1;
            if let Some(fit) = episode.comm_fitness_accum.get_mut(i) { *fit += REPRO_BIRTH_FITNESS_PARENT; }
        }
        if let Some(pb) = episode.agents.get_mut(j) {
            pb.energy = (pb.energy - cost * 0.5).max(0.0);
            pb.repro_cooldown = ECO_BIRTH_COOLDOWN_STEPS;
            pb.offspring_count += 1;
            if let Some(fit) = episode.comm_fitness_accum.get_mut(j) { *fit += REPRO_BIRTH_FITNESS_PARENT; }
        }
    }
}

// draw_text_clamped moved to ui_common::draw_text_clamped
