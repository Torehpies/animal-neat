//! Visualize Ecosystem example
//!
//! High-level flow:
//! - AppState holds the evolving NEAT population plus visualization flags.
//! - Each generation, we evaluate genomes over EPISODES_PER_GEN episodes.
//! - Fitness combines intake (plants/meat), exploration, survival, and optional comm rewards.
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
#[path = "visualize_ecosystem/ui/graphs.rs"]
mod ui_graphs;
#[path = "visualize_ecosystem/elements/body.rs"]
mod body;
#[path = "visualize_ecosystem/sim/mod.rs"]
mod sim;
use sim::{Episode, Agent, AgentId};
use ::rand::Rng;
use params::*;
use sim::eval_population_single_episode;
use ui_menu::{SimConfig, MenuState, draw_menu};


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
            show_graphs_panel: true,
            graphs: ui_graphs::Trends::new(1024),
            sim_config,
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
            let filename = format!("snapshots/auto_pop_snapshot_gen{:0>6}.json", self.generation);
            if let Err(e) = io::save_population_snapshot(&filename, self.generation, &self.population, &self.innov) {
                eprintln!("Auto-snapshot failed: {e}");
            } else {
                println!("Auto-saved population snapshot to {filename}");
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
    // Main loop that can restart with new config
    'main_loop: loop {
        // Show menu first to get configuration
        let mut menu_state = MenuState::new();
        let sim_config = loop {
            if let Some(config) = draw_menu(&mut menu_state) {
                break config;
            }
            next_frame().await;
        };
        
        // Apply configuration to global params (via world module)
        world::set_runtime_config(sim_config.world_width, sim_config.world_height, sim_config.max_food, sim_config.food_respawn_prob);
        params::set_runtime_energy_config(sim_config.initial_energy, sim_config.max_energy, sim_config.energy_drain_per_step);
        params::set_runtime_population_size(sim_config.population_size);
        
        let mut state = AppState::new(sim_config);
        let mut running = true;      // continuous evolution by default
        let mut fast_mode = false;   // start at normal speed
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
    if is_key_pressed(KeyCode::Z) { state.show_graphs_panel = !state.show_graphs_panel; }
        
        // ESC: if focused on an agent, clear focus; otherwise go back to menu
        if is_key_pressed(KeyCode::Escape) {
            if state.focused_agent.is_some() {
                state.focused_agent = None;
            } else {
                // Go back to menu
                continue 'main_loop;
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
                    // Update graphs trends per step
                    let alive_ct = state.episode.agents.iter().filter(|a| a.energy > 0.0).count() as f32;
                    let species_ct = state.speciator.get_species().len() as f32;
                    let deaths_ct = state.episode.agents.iter().filter(|a| a.energy <= 0.0 && !a.consumed).count() as f32;
                    state.graphs.pop.push(alive_ct);
                    state.graphs.species.push(species_ct);
                    state.graphs.births.push(state.episode.births_this_episode as f32);
                    state.graphs.deaths.push(deaths_ct);
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
                    // Push fitness trends after evaluation window
                    if state.last_best.is_finite() { state.graphs.best.push(state.last_best); state.graphs.mean.push(state.last_avg); }
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
                        // Push fitness trends after evaluation window
                        if state.last_best.is_finite() { state.graphs.best.push(state.last_best); state.graphs.mean.push(state.last_avg); }
                    } else {
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
        let mut chosen_species: Vec<usize> = species_best.into_iter().take(k).map(|(sid, _)| sid).collect();

        // Compute base quota and distribute remainder to top species
    let pop_cap = params::get_population_size();
    let base = pop_cap / k;
    let mut remainder = pop_cap % k;

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
    let child_species = *member_species.get(population.len()-1).unwrap_or(&sid);

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
