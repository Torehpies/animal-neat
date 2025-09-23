use macroquad::prelude::*;
use neat::neat::{
    config::EvolutionConfig,
    evolution,
    genome::Genome,
    innovation_tracker::InnovationTracker,
    speciator::Speciator,
};
use neat::neat::species::Species;
use std::collections::VecDeque;
use ::rand::Rng;
#[path = "visualize_ecosystem/params.rs"]
mod params;
#[path = "visualize_ecosystem/sensing.rs"]
mod sensing;
#[path = "visualize_ecosystem/world.rs"]
mod world;
#[path = "visualize_ecosystem/sim.rs"]
mod sim;
#[path = "visualize_ecosystem/ui/common.rs"]
mod ui_common;
#[path = "visualize_ecosystem/ui/world_view.rs"]
mod ui_world_view;
#[path = "visualize_ecosystem/ui/hud.rs"]
mod ui_hud;
#[path = "visualize_ecosystem/ui/network.rs"]
mod ui_network;

// Use centralized params
use params::*;

#[derive(Clone, Copy, Debug)]
struct Vec2 { x: f32, y: f32 }

#[allow(dead_code)]
impl Vec2 {
    fn new(x: f32, y: f32) -> Self { Self { x, y } }
    fn add(self, o: Self) -> Self { Self::new(self.x + o.x, self.y + o.y) }
    fn sub(self, o: Self) -> Self { Self::new(self.x - o.x, self.y - o.y) }
    fn mul(self, s: f32) -> Self { Self::new(self.x * s, self.y * s) }
    fn dot(self, o: Self) -> f32 { self.x * o.x + self.y * o.y }
    fn length(self) -> f32 { self.dot(self).sqrt() }
    fn normalized(self) -> Self { let len = self.length().max(1e-6); Self::new(self.x/len, self.y/len) }
    fn clamp_to_world(self) -> Self { Self::new(self.x.clamp(0.0, WORLD_W), self.y.clamp(0.0, WORLD_H)) }
}

// world.rs now provides rand_pos/build_world/food growth helpers

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
struct AgentId(usize);

#[derive(Clone, Debug)]
struct Agent {
    id: AgentId,
    pos: Vec2,
    theta: f32,
    energy: f32,
    eaten: usize,
    consumed: bool, // true if this agent's body has been eaten and removed from world
    kills: usize,                // number of agents eaten (live or dead) in this episode
    predation_flash_steps: u16,  // visual cue counter for recent predation
    dead_since: Option<usize>,   // step index when this agent died
    corpse_energy: f32,          // remaining energy in corpse (for scavenging)
    digest: VecDeque<DigestEvent>, // incoming energy deliveries
    last_food_mem: Vec2,         // memory of last step's nearest-food local vector
    last_danger_mem: Vec2,       // memory of last step's nearest-agent local vector
}

#[derive(Clone, Copy, Debug)]
struct DigestEvent { remaining: u16, per_step: f32 }

struct Episode {
    food: Vec<Vec2>,
    agents: Vec<Agent>,
    steps: usize,
    first_eat_step: Option<usize>,
    // Motor usage stats (for HUD): aggregated over agent-steps this episode
    total_agent_steps: usize,
    avg_speed_accum: f32,
    heading_change_accum: f32,
}

fn dir_from_theta(theta: f32) -> Vec2 { Vec2 { x: theta.cos(), y: theta.sin() } }


// sample_inputs replaced by build_inputs in Phase 3

fn grid_index(p: Vec2) -> u32 {
    let nx = (WORLD_W / EXPL_CELL_SIZE).ceil() as u32;
    let ix = (p.x / EXPL_CELL_SIZE).floor().clamp(0.0, nx as f32 - 1.0) as u32;
    let iy = (p.y / EXPL_CELL_SIZE).floor().clamp(0.0, (WORLD_H / EXPL_CELL_SIZE).ceil() as f32 - 1.0) as u32;
    iy * nx + ix
}

// eat_if_near moved to world.rs

fn eval_population_single_episode(population: &[Genome]) -> Vec<f32> {
    let mut rng = ::rand::rng();
    let mut food = world::build_world(&mut rng);
    let mut agents: Vec<Agent> = population.iter().enumerate().map(|(i, _)| Agent {
        id: AgentId(i),
    pos: world::rand_pos(&mut rng),
        theta: -std::f32::consts::FRAC_PI_2,
        energy: INITIAL_ENERGY,
        eaten: 0,
        consumed: false,
        kills: 0,
        predation_flash_steps: 0,
        dead_since: None,
        corpse_energy: 0.0,
        digest: VecDeque::new(),
        last_food_mem: Vec2 { x: 0.0, y: 0.0 },
        last_danger_mem: Vec2 { x: 0.0, y: 0.0 },
    }).collect();
    // Track exploration (unique grid cells); shaping buckets removed for simplification
    let mut visited: Vec<std::collections::HashSet<u32>> = vec![std::collections::HashSet::new(); agents.len()];

    let mut steps = 0usize;
    // (Removed approach/spin shaping trackers)
    while steps < MAX_STEPS {
        if agents.iter().all(|a| a.energy <= 0.0) { break; }
        // Snapshot for predation decisions to avoid borrow conflicts
        let snapshot: Vec<(Vec2, bool, bool)> = agents.iter().map(|a| (a.pos, a.energy > 0.0, a.consumed)).collect();
        let mut prey_targets: Vec<Option<usize>> = vec![None; agents.len()];
        for (i, a) in agents.iter_mut().enumerate() {
            if a.energy <= 0.0 { continue; }
            // Build extended inputs (ray-first): per-ray signals + energy + memory + density
            let (cur_fx, cur_fy) = sensing::food_vector_from_rays(a.pos, a.theta, &food);
            let density = sensing::density_sectors(a.pos, a.theta, &snapshot, i);
            // Digest before acting (shared)
            sim::apply_digestion(a);
            visited[i].insert(grid_index(a.pos));
            let energy_in = (a.energy / INITIAL_ENERGY).clamp(0.0, 1.0);
            let inputs = sensing::build_inputs(a.pos, a.theta, &food, energy_in, a.last_food_mem, a.last_danger_mem, &density, &snapshot, i);
            let out = population[i].evaluate_slice(&inputs);
            // Vector drive: two outputs -> (vx, vy) in [-1,1]
            let mut vx = out.get(0).copied().unwrap_or(0.0);
            let mut vy = out.get(1).copied().unwrap_or(0.0);
            // Noise
            vx += rng.random_range(-MOTOR_NOISE..MOTOR_NOISE);
            vy += rng.random_range(-MOTOR_NOISE..MOTOR_NOISE);
            vx = vx.clamp(-1.0, 1.0);
            vy = vy.clamp(-1.0, 1.0);
            let mut speed = (vx * vx + vy * vy).sqrt();
            let mut dir = if speed > 1e-4 { Vec2 { x: vx / speed, y: vy / speed } } else { dir_from_theta(a.theta) };
            if speed > 1.0 { speed = 1.0; }
            // Smooth heading toward desired direction if enabled
            if SMOOTH_HEADING && speed > 1e-4 {
                let desired = vy.atan2(vx); // atan2(y, x)
                let mut delta = desired - a.theta;
                // wrap to (-PI, PI]
                while delta > std::f32::consts::PI { delta -= 2.0 * std::f32::consts::PI; }
                while delta <= -std::f32::consts::PI { delta += 2.0 * std::f32::consts::PI; }
                let limited = delta.clamp(-MAX_HEADING_DELTA, MAX_HEADING_DELTA);
                a.theta += limited;
                // recompute direction from updated heading for movement & sensors
                dir = dir_from_theta(a.theta);
            } else if speed > 1e-4 {
                // Instant heading mode (if SMOOTH_HEADING == false)
                a.theta = vy.atan2(vx);
                dir = dir_from_theta(a.theta);
            }
            let vel = dir.mul(speed * MAX_SPEED);
            let prev = a.pos;
            a.pos = a.pos.add(vel).clamp_to_world();
            if world::eat_along_path(&mut food, prev, a.pos) || world::eat_if_near(&mut food, a.pos) {
                if DIGEST_STEPS_PLANT > 0 { a.digest.push_back(DigestEvent { remaining: DIGEST_STEPS_PLANT, per_step: FOOD_ENERGY / (DIGEST_STEPS_PLANT as f32) }); }
                else { a.energy = (a.energy + FOOD_ENERGY).min(INITIAL_ENERGY); }
                a.eaten += 1; }
            // Predation/scavenging: choose a nearby target (record only)
            if PREDATION_ENABLED || SCAVENGE_ENABLED {
                let mut target: Option<usize> = None;
                for j in 0..snapshot.len() {
                    if j == i { continue; }
                    let (pos_j, alive_j, consumed_j) = snapshot[j];
                    if consumed_j { continue; }
                    if (alive_j && !PREDATION_ENABLED) || (!alive_j && !SCAVENGE_ENABLED) { continue; }
                    let dx = pos_j.x - a.pos.x; let dy = pos_j.y - a.pos.y;
                    if (dx*dx + dy*dy).sqrt() <= EAT_AGENT_RADIUS { target = Some(j); break; }
                }
                prey_targets[i] = target;
            }
            // Energy cost: base + movement + optional turning if smoothing
            let mut energy_cost = ENERGY_DRAIN_PER_STEP + speed * MOVE_ENERGY_SCALE;
            if SMOOTH_HEADING && speed > 1e-4 {
                // approximate used heading fraction by how much direction deviated this step
                // (simplified: constant small turn cost when moving)
                energy_cost += TURN_ENERGY_SCALE;
            }
            a.energy -= energy_cost;
            if a.energy <= 0.0 { if a.dead_since.is_none() { a.dead_since = Some(steps); a.corpse_energy = CORPSE_INITIAL_ENERGY; } }
            // Update memories after acting
            a.last_food_mem = Vec2 { x: cur_fx, y: cur_fy };
            let (dx_mem, dy_mem) = sensing::nearest_agent_vector_local(a.pos, a.theta, &snapshot, i);
            a.last_danger_mem = Vec2 { x: dx_mem, y: dy_mem };
            // (Removed approach reward & spin penalty accumulation)
        }
        // Resolve predation and tick corpse/flash decay (shared)
        sim::resolve_predation(&mut agents, &prey_targets, steps);
        sim::decay_corpses_and_flashes(&mut agents);
        // (Removed avoidance/crowding penalty sampling)
        // Plants grow/spread over time (season-aware)
        world::set_current_step(steps);
        world::food_growth_step(&mut food, &mut rng);
        steps += 1;
    }

    // Compose final fitness with optional normalized exploration and sublinear eaten term
    let total_cells = ((WORLD_W / EXPL_CELL_SIZE).ceil() * (WORLD_H / EXPL_CELL_SIZE).ceil()) as f32;
    agents.iter().enumerate().map(|(i, a)| {
        let eaten_plants = a.eaten.saturating_sub(a.kills) as f32;
        let eaten_meat = a.kills as f32;
        let intake = eaten_plants * PLANT_FITNESS + eaten_meat * MEAT_FITNESS;
        let frac = if total_cells > 0.0 { (visited[i].len() as f32) / total_cells } else { 0.0 };
        let exploration = frac * EXPL_WEIGHT;
        intake + exploration + (steps as f32) * SURVIVAL_STEP_FITNESS
    }).collect()
}

impl Episode {
    fn new<R: Rng>(rng: &mut R, agent_count: usize) -> Self {
        let mut agents = Vec::with_capacity(agent_count);
        for i in 0..agent_count {
            agents.push(Agent {
                id: AgentId(i),
                pos: world::rand_pos(rng),
                theta: -std::f32::consts::FRAC_PI_2,
                energy: INITIAL_ENERGY,
                eaten: 0,
                consumed: false,
                kills: 0,
                predation_flash_steps: 0,
                dead_since: None,
                corpse_energy: 0.0,
                digest: VecDeque::new(),
                last_food_mem: Vec2 { x: 0.0, y: 0.0 },
                last_danger_mem: Vec2 { x: 0.0, y: 0.0 },
            });
        }
    Self {
        food: world::build_world(rng),
        agents,
        steps: 0,
        first_eat_step: None,
    total_agent_steps: 0,
    avg_speed_accum: 0.0,
    heading_change_accum: 0.0,
    }
    }

    fn step<R: Rng>(&mut self, population: &[Genome], rng: &mut R) -> bool {
        // Live episode: continue until all agents are dead (ignore max steps and food exhaustion)
        if self.agents.iter().all(|a| a.energy <= 0.0) { return false; }
        // Snapshot for predation decisions
        let snapshot: Vec<(Vec2, bool, bool)> = self.agents.iter().map(|a| (a.pos, a.energy > 0.0, a.consumed)).collect();
        let mut prey_targets: Vec<Option<usize>> = vec![None; self.agents.len()];
        for (i, a) in self.agents.iter_mut().enumerate() {
            if a.energy <= 0.0 { continue; }
            // Build extended inputs (Phase 3): rays + current food vec + energy + memory + density
            let (cur_fx, cur_fy) = sensing::food_vector_from_rays(a.pos, a.theta, &self.food);
            let (_cur_dx, _cur_dy) = sensing::nearest_agent_vector_local(a.pos, a.theta, &snapshot, i);
            let density = sensing::density_sectors(a.pos, a.theta, &snapshot, i);
            // Digestive intake before action (shared)
            sim::apply_digestion(a);
            let energy_in = (a.energy / INITIAL_ENERGY).clamp(0.0, 1.0);
            let inputs = sensing::build_inputs(a.pos, a.theta, &self.food, energy_in, a.last_food_mem, a.last_danger_mem, &density, &snapshot, i);
            let out = population[a.id.0].evaluate_slice(&inputs);
            // Vector drive (live episode): outputs -> (vx, vy)
            let mut rng_local = ::rand::rng();
            let mut vx = out.get(0).copied().unwrap_or(0.0) + rng_local.random_range(-MOTOR_NOISE..MOTOR_NOISE);
            let mut vy = out.get(1).copied().unwrap_or(0.0) + rng_local.random_range(-MOTOR_NOISE..MOTOR_NOISE);
            vx = vx.clamp(-1.0, 1.0); vy = vy.clamp(-1.0, 1.0);
            let mut speed = (vx*vx + vy*vy).sqrt();
            if speed > 1.0 { speed = 1.0; }
            let mut heading_delta_used = 0.0f32;
            if SMOOTH_HEADING && speed > 1e-4 {
                let desired = vy.atan2(vx);
                let mut delta = desired - a.theta;
                while delta > std::f32::consts::PI { delta -= 2.0 * std::f32::consts::PI; }
                while delta <= -std::f32::consts::PI { delta += 2.0 * std::f32::consts::PI; }
                let limited = delta.clamp(-MAX_HEADING_DELTA, MAX_HEADING_DELTA);
                a.theta += limited;
                heading_delta_used = limited.abs();
            } else if speed > 1e-4 {
                a.theta = vy.atan2(vx);
            }
            let dir = dir_from_theta(a.theta);
            let vel = dir.mul(speed * MAX_SPEED);
            let prev = a.pos;
            a.pos = a.pos.add(vel).clamp_to_world();
            // eat along the path (continuous) to prevent tunneling; fallback to near check
            if world::eat_along_path(&mut self.food, prev, a.pos) || world::eat_if_near(&mut self.food, a.pos) {
                if DIGEST_STEPS_PLANT > 0 { a.digest.push_back(DigestEvent { remaining: DIGEST_STEPS_PLANT, per_step: FOOD_ENERGY / (DIGEST_STEPS_PLANT as f32) }); }
                else { a.energy = (a.energy + FOOD_ENERGY).min(INITIAL_ENERGY); }
                a.eaten += 1;
                if self.first_eat_step.is_none() { self.first_eat_step = Some(self.steps); }
            }
            // Predation/scavenging: choose a target to apply after the loop
            if PREDATION_ENABLED || SCAVENGE_ENABLED {
                let mut target: Option<usize> = None;
                for j in 0..snapshot.len() {
                    if j == i { continue; }
                    let (pos_j, alive_j, consumed_j) = snapshot[j];
                    if consumed_j { continue; }
                    if (alive_j && !PREDATION_ENABLED) || (!alive_j && !SCAVENGE_ENABLED) { continue; }
                    let dx = pos_j.x - a.pos.x; let dy = pos_j.y - a.pos.y;
                    if (dx*dx + dy*dy).sqrt() <= EAT_AGENT_RADIUS { target = Some(j); break; }
                }
                prey_targets[i] = target;
            }
            // Energy & stats
            self.total_agent_steps += 1;
            self.avg_speed_accum += speed;
            if SMOOTH_HEADING { self.heading_change_accum += heading_delta_used; }
            let mut energy_cost = ENERGY_DRAIN_PER_STEP + speed * MOVE_ENERGY_SCALE;
            if SMOOTH_HEADING && speed > 1e-4 { energy_cost += TURN_ENERGY_SCALE; }
            a.energy -= energy_cost;
            if a.energy <= 0.0 { if a.dead_since.is_none() { a.dead_since = Some(self.steps); a.corpse_energy = CORPSE_INITIAL_ENERGY; } }
            // Update memories after acting
            a.last_food_mem = Vec2 { x: cur_fx, y: cur_fy };
            // For now, we only store danger memory; current danger not part of inputs to keep size down
            let (dx_mem, dy_mem) = sensing::nearest_agent_vector_local(a.pos, a.theta, &snapshot, i);
            a.last_danger_mem = Vec2 { x: dx_mem, y: dy_mem };
        }
        // Resolve predation after movement and decay (shared)
        sim::resolve_predation(&mut self.agents, &prey_targets, self.steps);
        sim::decay_corpses_and_flashes(&mut self.agents);
        // Plants grow/spread over time in the live world too (season-aware)
        world::set_current_step(self.steps);
        world::food_growth_step(&mut self.food, rng);
        // Decay predation flash counters
        for a in &mut self.agents {
            if a.predation_flash_steps > 0 { a.predation_flash_steps -= 1; }
        }
        self.steps += 1; true
    }

    fn is_finished(&self) -> bool {
        // Only finish when all agents are dead
        self.agents.iter().all(|a| a.energy <= 0.0)
    }
}

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
    last_species: Vec<Species>,
    // Best-ever genome tracking for HUD network rendering
    best_ever_fitness: f32,
    best_ever_generation: usize,
    best_ever_genome: Option<Genome>,
    // Best of last evaluated generation
    last_best_generation: usize,
    last_best_genome: Option<Genome>,
    // Debug overlays
    show_density_overlay: bool,
    show_vector_overlay: bool,
}

impl AppState {
    fn new(pop_size: usize) -> Self {
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
        let episode = Episode::new(&mut rng, pop_size);
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
            last_species: Vec::new(),
            best_ever_fitness: f32::NEG_INFINITY,
            best_ever_generation: 0,
            best_ever_genome: None,
            last_best_generation: 0,
            last_best_genome: None,
            show_density_overlay: false,
            show_vector_overlay: false,
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
        if let Some((best_idx, &fit)) = fitness_scores
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
            if fit > self.best_ever_fitness {
                self.best_ever_fitness = fit;
                self.best_ever_generation = self.generation;
                if best_idx < self.population.len() {
                    self.best_ever_genome = Some(self.population[best_idx].clone());
                }
            }
        }
        self.population = evolution::evolution(
            std::mem::take(&mut self.population),
            fitness_scores,
            &mut self.speciator,
            &mut self.innov,
            &self.cfg,
        );
        // snapshot species from evaluated generation for HUD
        self.last_species = self.speciator.get_species().clone();
        self.generation += 1;
        let mut rng = ::rand::rng();
        self.episode = Episode::new(&mut rng, self.population.len());
        // speciate new population for coloring and update mapping
        self.speciator.speciate(&self.population);
        self.member_species = {
            let mut map = vec![0usize; self.population.len()];
            for (sidx, s) in self.speciator.get_species().iter().enumerate() {
                for &m in &s.members { if m < self.population.len() { map[m] = sidx; } }
            }
            map
        };
    }
}

use ui_common::screen_to_world;

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
    let mut state = AppState::new(params::POPULATION_SIZE);
    let mut running = true;      // continuous evolution by default
    let mut fast_mode = false;   // start at normal speed
    let mut normal_step_timer = 0.0f32;          // accumulates frame time for normal stepping
    let normal_step_interval = 0.02f32;           // seconds per simulation step in normal mode
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
        if is_key_pressed(KeyCode::R) { let mut rng = ::rand::rng(); state.episode = Episode::new(&mut rng, state.population.len()); }
        if is_key_pressed(KeyCode::V) { state.show_cones = !state.show_cones; }
    if is_key_pressed(KeyCode::D) { state.show_density_overlay = !state.show_density_overlay; }
    if is_key_pressed(KeyCode::B) { state.show_vector_overlay = !state.show_vector_overlay; }
        // Removed population size controls

        if running {
            let mut rng = ::rand::rng();
            if fast_mode {
                // Run many simulation steps per frame until the episode finishes, then evolve
                for _ in 0..fast_steps_per_frame {
                    if state.episode.is_finished() {
                        state.evolve_one_generation();
                        state.episode = Episode::new(&mut rng, state.population.len());
                        break;
                    }
                    state.episode.step(&state.population, &mut rng);
                }
                // If it finished exactly on the last step, evolve now
                if state.episode.is_finished() {
                    state.evolve_one_generation();
                    state.episode = Episode::new(&mut rng, state.population.len());
                }
            } else {
                // Normal mode: advance one simulation step per second
                normal_step_timer += get_frame_time();
                if normal_step_timer >= normal_step_interval {
                    normal_step_timer -= normal_step_interval;
                    if state.episode.is_finished() {
                        state.evolve_one_generation();
                        state.episode = Episode::new(&mut rng, state.population.len());
                    } else {
                        state.episode.step(&state.population, &mut rng);
                        if state.episode.is_finished() {
                            state.evolve_one_generation();
                            state.episode = Episode::new(&mut rng, state.population.len());
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

    ui_world_view::draw_world(
        world_area,
        &state.episode,
        state.show_cones,
        &state.member_species,
        state.show_density_overlay,
        state.show_vector_overlay,
        mouse_world,
    );
    ui_hud::draw_hud(hud_area, &state, running, fast_mode, &state.member_species);

        next_frame().await
    }
}

// draw_text_clamped moved to ui_common::draw_text_clamped
