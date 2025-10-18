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
use crate::sim::AgentKind;
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
    // Graphs
    show_graphs_panel: bool,
    graphs: ui_graphs::Trends,
    // Runtime config
    #[allow(dead_code)]
    pub sim_config: SimConfig,
    // Per-session save prefix (unique per new simulation)
    pub save_prefix: String,
    // Diagnostics
    show_fps: bool,
    ultra_mode: bool,
    // Scoreboard modal
    show_scoreboard_panel: bool,  // user toggle (T): enable/disable pause + scoreboard at episode end, default off
    scoreboard_pending: bool,     // scoreboard is currently open (episode ended and we're paused)
    scoreboard_rows: Vec<ScoreEntry>,
    // HUD quick-save feedback
    hud_toast: Option<(String, f32)>, // (message, remaining_secs)
    hud_save_cooldown: f32,
}

#[derive(Clone, Debug)]
pub struct ScoreEntry {
    idx: usize,
    species: usize,
    score: f32,
    eaten_plants: i32,
    eaten_meat: i32,
    offspring: usize,
    alive_steps: u32,
    idle_penalty_value: f32,
    herd_value: f32,
    approach_value: f32,
    chase_value: f32,
    chase_same_value: f32,
    attack_hits: usize,
    kills_caused: usize,
    avg_energy_norm: f32,
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
            show_cones: false,
            member_species,
            last_best_generation: 0,
            last_best_genome: None,
            show_unified_overlay: false,
            show_energy_overlay: false,
            show_collision_radii: false,
            show_grid: false,
            show_best_network_panel: false,
            show_live_network: false,
            focused_agent: None,
            eco_episode_counter: 0,
            show_controls: true,
            color_by_species: true,
            show_graphs_panel: false,
            graphs: ui_graphs::Trends::new(),
            sim_config,
            save_prefix,
            show_fps: true,
            ultra_mode: false,
            show_scoreboard_panel: false, // default: autoplay between episodes (no pause)
            scoreboard_pending: false,    // no scoreboard open
            scoreboard_rows: Vec::new(),
            hud_toast: None,
            hud_save_cooldown: 0.0,
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
        params::set_runtime_energy_config(sim_config.initial_energy, sim_config.max_energy, sim_config.energy_drain_per_step);
        params::set_runtime_population_size(sim_config.population_size);
        // Apply fitness weights before starting
        params::set_fitness_weights(
            sim_config.w_lifetime,
            sim_config.w_energy,
            sim_config.w_offspring,
            sim_config.w_comm,
            sim_config.w_idle_penalty,
            sim_config.w_plant,
            sim_config.w_meat,
            sim_config.w_attacks,
            sim_config.w_kills,
            sim_config.w_herding,
        );
        params::set_behavior_weights(
            sim_config.w_approach,
            sim_config.w_chase,
            sim_config.w_chase_same,
            sim_config.w_herding,
            sim_config.w_attacks,
            sim_config.w_kills,
            sim_config.w_plant,
            sim_config.w_meat,
            sim_config.w_idle_penalty,
        );
        
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
            if is_key_pressed(KeyCode::P) { running = !running; }
            if is_key_pressed(KeyCode::F) { fast_mode = !fast_mode; }
            if is_key_pressed(KeyCode::R) { let mut rng = ::rand::rng(); state.episode = Episode::new(&mut rng, state.population.len(), &state.member_species); }
        if is_key_pressed(KeyCode::V) { state.show_cones = !state.show_cones; }
            if is_key_pressed(KeyCode::U) { state.show_unified_overlay = !state.show_unified_overlay; }
            if is_key_pressed(KeyCode::E) { state.show_energy_overlay = !state.show_energy_overlay; }
        // 'C' key: if scoreboard is open, it acts as Continue; otherwise it's for collision radii toggle
        if is_key_pressed(KeyCode::C) {
            if state.scoreboard_pending {
                let mut rng = ::rand::rng();
                finalize_end_of_episode(&mut state, &mut rng);
                running = true; // resume autoplay
            } else {
                state.show_collision_radii = !state.show_collision_radii;
            }
        }
        if is_key_pressed(KeyCode::G) { state.show_grid = !state.show_grid; }
        if is_key_pressed(KeyCode::N) { state.show_best_network_panel = !state.show_best_network_panel; }
        if is_key_pressed(KeyCode::M) { state.show_live_network = !state.show_live_network; }
        if is_key_pressed(KeyCode::H) { state.show_controls = !state.show_controls; }
        if is_key_pressed(KeyCode::K) { state.color_by_species = !state.color_by_species; }
        if is_key_pressed(KeyCode::Z) { state.show_graphs_panel = !state.show_graphs_panel; }
        if is_key_pressed(KeyCode::X) { state.ultra_mode = !state.ultra_mode; }
        // Scoreboard toggle (T):
        // - Pressing T toggles the preference: when ON, the app pauses at episode end and shows the scoreboard;
        //   when OFF, episodes autoplay between generations and the scoreboard is not shown.
        // - When the scoreboard is currently open, T only toggles the preference for future episodes; it does NOT close or continue.
        if is_key_pressed(KeyCode::T) {
            if !state.scoreboard_pending {
                state.show_scoreboard_panel = !state.show_scoreboard_panel;
            } else {
                // Toggle preference only; keep scoreboard open until 'C' or button click
                state.show_scoreboard_panel = !state.show_scoreboard_panel;
            }
        }
        // Save snapshot (S) and Load most-recent snapshot (O)
        if is_key_pressed(KeyCode::S) {
            // Quick-save (overwrite) per-session file
            let filename = format!("{}__quicksave.json", state.save_prefix);
            if let Err(e) = snapshot::save_sim_snapshot(&filename, state.generation, &state.population, &state.innov, &state.episode, &state.member_species) {
                eprintln!("Failed to quick-save sim snapshot: {}", e);
                state.hud_toast = Some((format!("Save failed: {}", e), 3.5));
            } else {
                println!("Quick-saved sim snapshot to {}", filename);
                state.hud_toast = Some((format!("Saved: {}", filename), 2.5));
            }
        }
        if is_key_pressed(KeyCode::L) {
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
                        // rebuild speciator and member_species mapping
                        state.speciator.get_species_mut().clear();
                        state.speciator.speciate(&state.population);
                        state.member_species = snap.member_species;
                        println!("Loaded sim snapshot from {}", path.display());
                        state.hud_toast = Some((format!("Loaded: {}", path.display()), 2.5));
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
    // 'O' key: toggle FPS pill in HUD
    if is_key_pressed(KeyCode::O) { state.show_fps = !state.show_fps; }
        
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
                            if !state.scoreboard_pending { prepare_scoreboard(&mut state); }
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
                        state.graphs.pop.push(alive_ct);
                        state.graphs.species.push(species_ct);
                        state.graphs.births.push(state.episode.births_this_episode as f32);
                        state.graphs.deaths.push(deaths_ct);
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
                            if !state.scoreboard_pending { prepare_scoreboard(&mut state); }
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
                    state.graphs.pop.push(alive_ct);
                    state.graphs.species.push(species_ct);
                    state.graphs.births.push(state.episode.births_this_episode as f32);
                    state.graphs.deaths.push(deaths_ct);
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
                        state.graphs.pop.push(alive_ct);
                        state.graphs.species.push(species_ct);
                        state.graphs.births.push(state.episode.births_this_episode as f32);
                        state.graphs.deaths.push(deaths_ct);
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
                                if !state.scoreboard_pending { prepare_scoreboard(&mut state); }
                            } else {
                                finalize_end_of_episode(&mut state, &mut rng);
                            }
                        }
                    } else {
                        // Episode already finished when we got here (very short episodes): handle boundary
                        if state.show_scoreboard_panel {
                            if !state.scoreboard_pending { prepare_scoreboard(&mut state); }
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
        );
    ui_hud::draw_hud(hud_area, &mut state, running, fast_mode);
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

    // modal menu handled by ui_sim_menu when ESC is pressed

        next_frame().await;
    } // end 'sim_loop
    } // end 'main_loop
}

// Compute episode scores similar to eco_cull, without side effects
fn compute_episode_score(a: &crate::sim::Agent, comm_fit: f32) -> (f32, i32, i32, f32, f32, f32, f32, f32, f32) {
    use crate::params::*;
    // Scoreboard score aligns with eval.rs weighted formula
    let avg_energy_norm = if a.alive_steps > 0 { (a.energy_accum / a.alive_steps as f32) / crate::params::get_max_energy() } else { 0.0 };
    let lifetime_score = (a.alive_steps as f32) / (MAX_STEPS as f32);
    let offspring_score = a.offspring_count as f32;
    let plants = a.eaten.saturating_sub(a.kills) as f32;
    let meat = a.kills as f32;
    // Normalize idle penalty so w_idle meaning is comparable to other [0,1] terms
    let max_idle_steps = (MAX_STEPS.saturating_sub(IDLENESS_THRESHOLD_STEPS)) as f32;
    let max_idle_penalty = max_idle_steps * IDLENESS_PENALTY_PER_STEP;
    let idle_penalty = if max_idle_penalty > 0.0 { (a.total_idle_penalty / max_idle_penalty).clamp(0.0, 1.0) } else { 0.0 };
    let w_life = crate::params::get_fit_lifetime_weight();
    let w_energy = crate::params::get_fit_energy_weight();
    let w_off = crate::params::get_fit_offspring_weight();
    let w_comm = crate::params::get_fit_comm_weight();
    let w_idle = crate::params::get_fit_idle_penalty_weight();
    let w_plant = crate::params::get_fit_plant_weight();
    let w_meat = crate::params::get_fit_meat_weight();
    let w_att = crate::params::get_fit_attacks_weight();
    let w_kill = crate::params::get_fit_kills_weight();
    let w_herd = crate::params::get_fit_herding_weight();
    let w_approach = crate::params::get_fit_approach_food_weight();
    let w_chase = crate::params::get_fit_chase_other_weight();
    let w_chase_same = crate::params::get_fit_chase_same_weight();
    let score = w_life * lifetime_score
        + w_energy * avg_energy_norm
        + w_off * offspring_score
        + w_comm * comm_fit
        - if IDLENESS_PENALTY_ENABLED { w_idle * idle_penalty } else { 0.0 }
        + w_plant * plants
        + w_meat * meat
        + w_att * (a.attack_hits as f32)
    + w_kill * (a.kills_caused as f32)
    + w_herd * a.herding_units
    + w_approach * a.approach_food_units
    + w_chase * a.chase_other_units
    + w_chase_same * a.chase_same_units;
    // Keep plant/meat counts for display only
    let eaten_plants = a.eaten.saturating_sub(a.kills) as i32;
    let eaten_meat = a.kills as i32;
    let idle_penalty_value = if IDLENESS_PENALTY_ENABLED { w_idle * idle_penalty } else { 0.0 };
    let herd_value = w_herd * a.herding_units;
    let approach_value = w_approach * a.approach_food_units;
    let chase_value = w_chase * a.chase_other_units;
    let chase_same_value = w_chase_same * a.chase_same_units;
    (score, eaten_plants, eaten_meat, avg_energy_norm, idle_penalty_value, herd_value, approach_value, chase_value, chase_same_value)
}

fn prepare_scoreboard(state: &mut AppState) {
    // Build rows and sort by score desc
    let mut rows: Vec<ScoreEntry> = Vec::with_capacity(state.episode.agents.len());
    for (i, a) in state.episode.agents.iter().enumerate() {
        let comm_fit = state.episode.comm_fitness_accum.get(i).copied().unwrap_or(0.0);
    let (score, plants, meat, avg_energy_norm, idle_penalty_value, herd_value, approach_value, chase_value, chase_same_value) = compute_episode_score(a, comm_fit);
        rows.push(ScoreEntry {
            idx: i,
            species: a.species_id,
            score,
            eaten_plants: plants,
            eaten_meat: meat,
            offspring: a.offspring_count,
            alive_steps: a.alive_steps,
            idle_penalty_value,
            herd_value,
            approach_value,
            chase_value,
            chase_same_value,
            attack_hits: a.attack_hits,
            kills_caused: a.kills_caused,
            avg_energy_norm,
        });
    }
    rows.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    state.scoreboard_rows = rows;
    state.scoreboard_pending = true; // scoreboard is open now; rendering is gated on show_scoreboard_panel
}

fn finalize_end_of_episode(state: &mut AppState, rng: &mut impl ::rand::Rng) {
    use crate::params::ECO_CONTINUOUS;
    // Compute intelligence proxy (from the just-finished episode) before resetting
    // Proxy focuses on behavior-shaping components: herding + approach/chase (other/same)
    let (intel_best, intel_mean) = {
        let w_herd = crate::params::get_fit_herding_weight();
        let w_approach = crate::params::get_fit_approach_food_weight();
        let w_chase = crate::params::get_fit_chase_other_weight();
        let w_chase_same = crate::params::get_fit_chase_same_weight();
        let mut best = f32::NEG_INFINITY;
        let mut sum = 0.0f32;
        let mut count = 0usize;
        for a in &state.episode.agents {
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
        state.episode = Episode::new(rng, state.population.len(), &state.member_species);
    } else {
        // Log intelligence proxy for this episode (before resetting)
        if intel_best.is_finite() { state.graphs.intel_best.push(intel_best); }
        if intel_mean.is_finite() { state.graphs.intel_mean.push(intel_mean); }
        state.evolve_one_generation();
        state.graphs.reset_episode();
        state.episode = Episode::new(rng, state.population.len(), &state.member_species);
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
        let w_life = crate::params::get_fit_lifetime_weight();
        let w_energy = crate::params::get_fit_energy_weight();
        let w_off = crate::params::get_fit_offspring_weight();
        let w_comm = crate::params::get_fit_comm_weight();
        let w_idle = crate::params::get_fit_idle_penalty_weight();
        let w_plant = crate::params::get_fit_plant_weight();
        let w_meat = crate::params::get_fit_meat_weight();
        let w_att = crate::params::get_fit_attacks_weight();
        let w_kill = crate::params::get_fit_kills_weight();
    let w_herd = crate::params::get_fit_herding_weight();
        let w_approach = crate::params::get_fit_approach_food_weight();
        let w_chase = crate::params::get_fit_chase_other_weight();
    let complexity_penalty = crate::params::COMPLEXITY_PENALTY_PER_CONN;
        state.episode.agents.iter().enumerate().map(|(i, a)| {
            let lifetime_score = (a.alive_steps as f32) / (MAX_STEPS as f32);
            let avg_energy_norm = if a.alive_steps > 0 { (a.energy_accum / a.alive_steps as f32) / crate::params::get_max_energy() } else { 0.0 };
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
                + w_chase * chase_units;
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
    let threshold = ECO_BIRTH_ENERGY_THRESHOLD;
    let mut eligible: Vec<usize> = Vec::new();
    for &i in &live_indices {
        let a = &episode.agents[i];
        let non_idle_ok = if params::ECO_REQUIRE_NON_IDLE_FOR_BIRTH {
            a.body.vel.length() > 0.2 || a.idle_steps < IDLENESS_THRESHOLD_STEPS
        } else { true };
        if a.repro_cooldown == 0 && a.offspring_count < ECO_MAX_OFFSPRING_PER_AGENT && a.energy >= threshold && non_idle_ok {
            eligible.push(i);
        }
    }

    // Attempt to find nearby pairs among eligible agents. Allow cross-species mating if genomes are similar enough by NEAT distance.
    let cost = ECO_BIRTH_ENERGY_COST;
    let mut births: Vec<(usize, usize, crate::body::Body, usize)> = Vec::new(); // (p1_idx, p2_idx, child_body, species_id_tag)
    let mut used: std::collections::HashSet<usize> = std::collections::HashSet::new();
    eligible.sort_unstable();
    'pairing: for (ii, &i) in eligible.iter().enumerate() {
        if used.contains(&i) { continue; }
        let ai = &episode.agents[i];
        for &j in eligible.iter().skip(ii+1) {
            if used.contains(&j) { continue; }
            let aj = &episode.agents[j];
            // Proximity check
            let dx = ai.body.pos.x - aj.body.pos.x; let dy = ai.body.pos.y - aj.body.pos.y;
            if dx*dx + dy*dy > ECO_MATE_RADIUS*ECO_MATE_RADIUS { continue; }
            // Similarity check: allow if same species OR compatibility distance <= threshold
            let mut similar = ai.species_id == aj.species_id;
            if !similar {
                let d = neat::neat::compatibility::distance(&population[i], &population[j], ECO_MATE_C1, ECO_MATE_C2, ECO_MATE_C3);
                if d <= ECO_MATE_COMPATIBILITY_THRESHOLD { similar = true; }
            }
            if !similar { continue; }
            // Energy cost feasibility
            let half = cost * 0.5; if ai.energy < half || aj.energy < half { continue; }
            // Child spawn near parents: midpoint with small jitter
            let mid = Vec2 { x: (ai.body.pos.x + aj.body.pos.x) * 0.5, y: (ai.body.pos.y + aj.body.pos.y) * 0.5 };
            let jitter = Vec2 { x: (rng.random::<f32>() - 0.5) * 3.0 * AGENT_RADIUS, y: (rng.random::<f32>() - 0.5) * 3.0 * AGENT_RADIUS };
            let mut pos = mid + jitter;
            pos.x = (pos.x % WORLD_W + WORLD_W) % WORLD_W; pos.y = (pos.y % WORLD_H + WORLD_H) % WORLD_H;
            // Tag child with one parent's species id for in-episode kin behavior. We choose the first parent's species id.
            let sid = ai.species_id;
            births.push((i, j, crate::body::Body { pos, vel: Vec2::new(0.0, 0.0), radius: AGENT_RADIUS }, sid));
            used.insert(i); used.insert(j);
            if live_indices.len() + births.len() >= ECO_MAX_POP { break 'pairing; }
        }
    }

    if births.is_empty() { return 0; }

    // Realize births: create child via crossover + mutation; debit both parents; set cooldowns; append agent + genome
    let mut realized = 0usize;
    for (i, j, body, sid) in births {
        // Double-check alignment: parents indices map to genome indices
        if i >= population.len() || j >= population.len() { continue; }
        let p1 = &population[i];
        let p2 = &population[j];
        // Offspring are produced by crossover from parents and immediately mutated (per request)
        let mut child_g = neat::neat::crossover::crossover(p1, p2);
        child_g.mutate(_innov, _cfg);
        population.push(child_g);

        // let child_species = *member_species.get(population.len()-1).unwrap_or(&sid); // unused currently

        // Decide newborn kind: inherit if parents share kind, otherwise random choice
        let child_kind = {
            let ai_kind = episode.agents[i].kind;
            let aj_kind = episode.agents[j].kind;
            if ai_kind == aj_kind { ai_kind } else { if rng.random::<f32>() < 0.5 { crate::sim::AgentKind::Herbivore } else { crate::sim::AgentKind::Carnivore } }
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
