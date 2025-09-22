use macroquad::prelude::*;
use neat::neat::{
    config::EvolutionConfig,
    evolution,
    genome::Genome,
    innovation_tracker::InnovationTracker,
    speciator::Speciator,
};
use neat::neat::species::Species;
use neat::neat::node_gene::NodeType;
use std::collections::VecDeque;
use ::rand::Rng;
#[path = "visualize_ecosystem/sensing.rs"]
mod sensing;
#[path = "visualize_ecosystem/world.rs"]
mod world;

// World/agent constants (continuous space)
const WORLD_W: f32 = 500.0;
const WORLD_H: f32 = 500.0;
const FOOD_COUNT: usize = 100;
const FOOD_RADIUS: f32 = 1.2;
const AGENT_RADIUS: f32 = 1.5;
const INITIAL_ENERGY: f32 = 300.0;
const ENERGY_DRAIN_PER_STEP: f32 = 0.4;
const FOOD_ENERGY: f32 = 75.0;
const MAX_STEPS: usize = 500;

// Plant/food dynamics
const MAX_FOOD: usize = 300;                 // hard cap on number of plants
const FOOD_MIN_SEP: f32 = 2.5;               // minimum separation between plants
const FOOD_RESPAWN_PROB: f32 = 0.06;         // per-step probability to spawn one random plant
const FOOD_SPREAD_CHANCE: f32 = 0.02;        // per existing plant, chance to spawn a nearby offshoot
const FOOD_SPREAD_RADIUS: f32 = 15.0;        // max radius for offshoot from parent plant

// Vision cone parameters
const VISION_RAYS: usize = 7;           // number of rays within the cone
const VISION_ANGLE_DEG: f32 = 90.0;     // total cone angle
const VISION_RANGE: f32 = 200.0;         // world units

// Movement
const MAX_TURN: f32 = std::f32::consts::PI / 15.0; // radians per step at full turn
const MAX_SPEED: f32 = 2.5;                        // units per step at full thrust

// Reduce circling: scale thrust down when turning strongly
const THRUST_TURN_COUPLING: f32 = 0.6; // 0 = none, 1 = full: thrust*(1-|turn|)

// Predation/scavenging
const EAT_AGENT_RADIUS: f32 = AGENT_RADIUS + AGENT_RADIUS;
const MEAT_ENERGY: f32 = 60.0;           // energy gained by eating an agent (alive or dead)
const PREDATION_ENABLED: bool = true;     // eat live agents when close
const SCAVENGE_ENABLED: bool = true;      // eat dead agents when close

// Additional sensing (Phase 3): danger vector memory and local density sectors
const DANGER_VECTOR_MAX_RANGE: f32 = 150.0; // range considered for nearest-agent danger vector
const DENSITY_SECTORS: usize = 8;           // angular sectors around the agent for density
const DENSITY_RADIUS: f32 = 40.0;           // neighborhood radius to accumulate density

const INPUTS: usize = VISION_RAYS * 2 + 3 + 4 + DENSITY_SECTORS; // per-ray [food, wall] + [food_vec x,y]+energy + [last_food x,y]+[last_danger x,y] + density[sectors]
const OUTPUTS: usize = 2; // turn, thrust

// Exploration and avoidance (mirrors headless)
const EXPL_CELL_SIZE: f32 = 10.0;
const EXPL_REWARD_PER_CELL: f32 = 0.02;
const AVOID_RADIUS: f32 = 3.0;
const AVOID_PENALTY_SCALE: f32 = 0.003;
const AVOID_CHECK_EVERY: usize = 2;
const EPISODES_PER_GEN: usize = 3; // average fitness over multiple randomized episodes

// Food-direction vector parameters
const FOOD_VECTOR_MAX_RANGE: f32 = 150.0; // how far we consider plants when building the vector

// Turn and fitness shaping
const TURN_COST: f32 = 0.02;     // energy drain per unit |turn|
const EAT_WEIGHT: f32 = 4.5;     // fitness weight for each item eaten (plants or agents)
const STEP_WEIGHT: f32 = 0.001;  // fitness weight per step survived

// Phase 2: corpse decay and digestive lag
const CORPSE_INITIAL_ENERGY: f32 = MEAT_ENERGY; // energy available in a fresh corpse
const CORPSE_DECAY_RATE: f32 = 0.02;            // fraction lost per step (e.g., 0.02 = 2%)
const DIGEST_STEPS_PLANT: u16 = 25;             // steps over which plant energy is released
const DIGEST_STEPS_MEAT: u16 = 35;              // steps over which meat energy is released

#[derive(Clone, Copy, Debug)]
struct Vec2 { x: f32, y: f32 }

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
    let mut visited: Vec<std::collections::HashSet<u32>> = vec![std::collections::HashSet::new(); agents.len()];
    let mut avoid_penalty: Vec<f32> = vec![0.0; agents.len()];

    let mut steps = 0usize;
    while steps < MAX_STEPS {
        if agents.iter().all(|a| a.energy <= 0.0) { break; }
        // Snapshot for predation decisions to avoid borrow conflicts
        let snapshot: Vec<(Vec2, bool, bool)> = agents.iter().map(|a| (a.pos, a.energy > 0.0, a.consumed)).collect();
        let mut prey_targets: Vec<Option<usize>> = vec![None; agents.len()];
        for (i, a) in agents.iter_mut().enumerate() {
            if a.energy <= 0.0 { continue; }
            // Build extended inputs (Phase 3): rays + current food vec + energy + memory + density
            let (cur_fx, cur_fy) = sensing::nearest_food_vector_local(a.pos, a.theta, &food);
            let density = sensing::density_sectors(a.pos, a.theta, &snapshot, i);
            // Digest before acting
            if !a.digest.is_empty() {
                let mut gained = 0.0f32;
                for ev in a.digest.iter_mut() { if ev.remaining > 0 { gained += ev.per_step; ev.remaining -= 1; } }
                a.digest.retain(|ev| ev.remaining > 0);
                a.energy = (a.energy + gained).min(INITIAL_ENERGY);
            }
            visited[i].insert(grid_index(a.pos));
            let energy_in = (a.energy / INITIAL_ENERGY).clamp(0.0, 1.0);
            let inputs = sensing::build_inputs(a.pos, a.theta, &food, energy_in, a.last_food_mem, a.last_danger_mem, &density);
            let out = population[i].evaluate_slice(&inputs);
            let turn = out.get(0).copied().unwrap_or(0.0).clamp(-1.0, 1.0);
            let thrust = out.get(1).copied().unwrap_or(0.0).clamp(0.0, 1.0);
            // Coupling: reduce thrust when turning strongly
            let thrust_eff = (thrust * (1.0 - THRUST_TURN_COUPLING * turn.abs())).clamp(0.0, 1.0);
            a.theta += turn * MAX_TURN;
            let dir = dir_from_theta(a.theta);
            let vel = dir.mul(thrust_eff * MAX_SPEED);
            a.pos = a.pos.add(vel).clamp_to_world();
            if world::eat_if_near(&mut food, a.pos) {
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
            a.energy -= ENERGY_DRAIN_PER_STEP + thrust_eff * 0.2 + TURN_COST * turn.abs();
            if a.energy <= 0.0 { if a.dead_since.is_none() { a.dead_since = Some(steps); a.corpse_energy = CORPSE_INITIAL_ENERGY; } }
            // Update memories after acting
            a.last_food_mem = Vec2 { x: cur_fx, y: cur_fy };
            let (dx_mem, dy_mem) = sensing::nearest_agent_vector_local(a.pos, a.theta, &snapshot, i);
            a.last_danger_mem = Vec2 { x: dx_mem, y: dy_mem };
        }
        // Resolve predation after movement without aliasing borrows
        let mut claimed = vec![false; agents.len()];
        for i in 0..agents.len() {
            if let Some(j) = prey_targets[i] {
                if claimed[j] || agents[j].consumed { continue; }
                let alive_j = agents[j].energy > 0.0;
                if (alive_j && !PREDATION_ENABLED) || (!alive_j && !SCAVENGE_ENABLED) { continue; }
                let gain = if alive_j { CORPSE_INITIAL_ENERGY } else { agents[j].corpse_energy.max(0.0) };
                agents[j].energy = 0.0;
                agents[j].dead_since.get_or_insert(steps);
                agents[j].corpse_energy = 0.0;
                agents[j].consumed = true;
                if gain > 0.0 {
                    if DIGEST_STEPS_MEAT > 0 { agents[i].digest.push_back(DigestEvent { remaining: DIGEST_STEPS_MEAT, per_step: gain / (DIGEST_STEPS_MEAT as f32) }); }
                    else { agents[i].energy = (agents[i].energy + gain).min(INITIAL_ENERGY); }
                }
                agents[i].eaten += 1;
                agents[i].kills += 1;
                agents[i].predation_flash_steps = agents[i].predation_flash_steps.saturating_add(10);
                claimed[j] = true;
            }
        }
        // Corpse decay
        for a in &mut agents {
            if a.energy <= 0.0 && !a.consumed && a.corpse_energy > 0.0 {
                a.corpse_energy *= (1.0 - CORPSE_DECAY_RATE).max(0.0);
                if a.corpse_energy < 0.1 { a.corpse_energy = 0.0; a.consumed = true; }
            }
        }
        // Decay predation flash counters
        for a in &mut agents {
            if a.predation_flash_steps > 0 { a.predation_flash_steps -= 1; }
        }
        if steps % AVOID_CHECK_EVERY == 0 {
            for i in 0..agents.len() {
                if agents[i].energy <= 0.0 { continue; }
                let pi = agents[i].pos;
                let mut pen = 0.0f32;
                for j in 0..agents.len() {
                    if i == j || agents[j].energy <= 0.0 { continue; }
                    let pj = agents[j].pos;
                    let dx = pj.x - pi.x; let dy = pj.y - pi.y; let d2 = dx*dx + dy*dy; let r2 = AVOID_RADIUS * AVOID_RADIUS;
                    if d2 < r2 { let d = d2.sqrt(); let m = (AVOID_RADIUS - d) / AVOID_RADIUS; pen += m * AVOID_PENALTY_SCALE; }
                }
                avoid_penalty[i] += pen;
            }
        }
        // Plants grow/spread over time
    world::food_growth_step(&mut food, &mut rng);
        steps += 1;
    }

    agents.iter().enumerate().map(|(i, a)| {
        let expl = visited[i].len() as f32 * EXPL_REWARD_PER_CELL;
        (a.eaten as f32) * EAT_WEIGHT + (steps as f32) * STEP_WEIGHT + expl - avoid_penalty[i]
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
    Self { food: world::build_world(rng), agents, steps: 0, first_eat_step: None }
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
            let (cur_fx, cur_fy) = sensing::nearest_food_vector_local(a.pos, a.theta, &self.food);
            let (_cur_dx, _cur_dy) = sensing::nearest_agent_vector_local(a.pos, a.theta, &snapshot, i);
            let density = sensing::density_sectors(a.pos, a.theta, &snapshot, i);
            // Digestive intake before action
            if !a.digest.is_empty() {
                let mut gained = 0.0f32;
                for ev in a.digest.iter_mut() {
                    if ev.remaining > 0 { gained += ev.per_step; ev.remaining -= 1; }
                }
                a.digest.retain(|ev| ev.remaining > 0);
                a.energy = (a.energy + gained).min(INITIAL_ENERGY);
            }
            let energy_in = (a.energy / INITIAL_ENERGY).clamp(0.0, 1.0);
            let inputs = sensing::build_inputs(a.pos, a.theta, &self.food, energy_in, a.last_food_mem, a.last_danger_mem, &density);
            let out = population[a.id.0].evaluate_slice(&inputs);
            let turn = out.get(0).copied().unwrap_or(0.0).clamp(-1.0, 1.0);
            let thrust = out.get(1).copied().unwrap_or(0.0).clamp(0.0, 1.0);
            // Coupling: reduce thrust when turning strongly
            let thrust_eff = (thrust * (1.0 - THRUST_TURN_COUPLING * turn.abs())).clamp(0.0, 1.0);
            a.theta += turn * MAX_TURN;
            let dir = dir_from_theta(a.theta);
            let vel = dir.mul(thrust_eff * MAX_SPEED);
            a.pos = a.pos.add(vel).clamp_to_world();
            // eat if close (sum of radii) with digestive lag
            if world::eat_if_near(&mut self.food, a.pos) {
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
            a.energy -= ENERGY_DRAIN_PER_STEP + thrust_eff * 0.2 + TURN_COST * turn.abs();
            if a.energy <= 0.0 { if a.dead_since.is_none() { a.dead_since = Some(self.steps); a.corpse_energy = CORPSE_INITIAL_ENERGY; } }
            // Update memories after acting
            a.last_food_mem = Vec2 { x: cur_fx, y: cur_fy };
            // For now, we only store danger memory; current danger not part of inputs to keep size down
            let (dx_mem, dy_mem) = sensing::nearest_agent_vector_local(a.pos, a.theta, &snapshot, i);
            a.last_danger_mem = Vec2 { x: dx_mem, y: dy_mem };
        }
        // Resolve predation after movement
        let mut claimed = vec![false; self.agents.len()];
        for i in 0..self.agents.len() {
            if let Some(j) = prey_targets[i] {
                if claimed[j] || self.agents[j].consumed { continue; }
                let alive_j = self.agents[j].energy > 0.0;
                if (alive_j && !PREDATION_ENABLED) || (!alive_j && !SCAVENGE_ENABLED) { continue; }
                // Determine meat energy to gain (decay for scavenging)
                let gain = if alive_j { CORPSE_INITIAL_ENERGY } else { self.agents[j].corpse_energy.max(0.0) };
                // Remove corpse
                self.agents[j].energy = 0.0;
                self.agents[j].dead_since.get_or_insert(self.steps);
                self.agents[j].corpse_energy = 0.0;
                self.agents[j].consumed = true;
                // Digestive lag for meat
                if gain > 0.0 {
                    if DIGEST_STEPS_MEAT > 0 { self.agents[i].digest.push_back(DigestEvent { remaining: DIGEST_STEPS_MEAT, per_step: gain / (DIGEST_STEPS_MEAT as f32) }); }
                    else { self.agents[i].energy = (self.agents[i].energy + gain).min(INITIAL_ENERGY); }
                }
                self.agents[i].eaten += 1;
                self.agents[i].kills += 1;
                self.agents[i].predation_flash_steps = self.agents[i].predation_flash_steps.saturating_add(10);
                if self.first_eat_step.is_none() { self.first_eat_step = Some(self.steps); }
                claimed[j] = true;
            }
        }
        // Corpse decay for dead but not yet consumed agents
        for a in &mut self.agents {
            if a.energy <= 0.0 && !a.consumed && a.corpse_energy > 0.0 {
                a.corpse_energy *= (1.0 - CORPSE_DECAY_RATE).max(0.0);
                if a.corpse_energy < 0.1 { a.corpse_energy = 0.0; a.consumed = true; }
            }
        }
        // Plants grow/spread over time in the live world too
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
    // Start with a lower threshold to encourage early splits; aim for ~5 species
    let mut speciator = Speciator::new(1.0).with_target(5, 0.05);
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

fn world_to_screen(area: Rect, p: Vec2) -> (f32, f32) {
    let sx = area.x + (p.x / WORLD_W) * area.w;
    let sy = area.y + (p.y / WORLD_H) * area.h;
    (sx, sy)
}

fn screen_to_world(area: Rect, sx: f32, sy: f32) -> Vec2 {
    let x = ((sx - area.x) / area.w).clamp(0.0, 1.0) * WORLD_W;
    let y = ((sy - area.y) / area.h).clamp(0.0, 1.0) * WORLD_H;
    Vec2 { x, y }
}

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (f32, f32, f32) {
    let h = (h % 1.0 + 1.0) % 1.0;
    if s <= 0.0 { return (v, v, v); }
    let i = (h * 6.0).floor();
    let f = h * 6.0 - i;
    let p = v * (1.0 - s);
    let q = v * (1.0 - s * f);
    let t = v * (1.0 - s * (1.0 - f));
    match i as i32 % 6 { 0 => (v, t, p), 1 => (q, v, p), 2 => (p, v, t), 3 => (p, q, v), 4 => (t, p, v), _ => (v, p, q) }
}

fn species_color(species_idx: usize) -> Color {
    let hue = ((species_idx as f32) * 0.618_033_988) % 1.0; // golden ratio spacing
    let (r, g, b) = hsv_to_rgb(hue, 0.65, 0.95);
    Color::new(r, g, b, 1.0)
}

fn draw_world(area: Rect, episode: &Episode, show_cones: bool, member_species: &[usize], show_density_overlay: bool, show_vector_overlay: bool, mouse_world: Option<Vec2>) {
    // background
    draw_rectangle(area.x, area.y, area.w, area.h, DARKGREEN);
    // border
    draw_rectangle_lines(area.x, area.y, area.w, area.h, 2.0, BLACK);
    // food
    for p in &episode.food {
        let (px, py) = world_to_screen(area, *p);
        let r = ((FOOD_RADIUS / WORLD_W) * area.w).max(2.0);
        draw_circle(px, py, r, YELLOW);
        // highlight if within eat range of any agent
        let eat_r = FOOD_RADIUS + AGENT_RADIUS;
        let mut near = false;
        for a in &episode.agents {
            let dx = p.x - a.pos.x; let dy = p.y - a.pos.y;
            let d2 = dx*dx + dy*dy; if d2 <= eat_r*eat_r { near = true; break; }
        }
        if near {
            draw_circle_lines(px, py, r + 2.0, 2.0, ORANGE);
        }
    }
    // Precompute snapshot for overlays
    let snapshot: Vec<(Vec2, bool, bool)> = episode.agents.iter().map(|a| (a.pos, a.energy > 0.0, a.consumed)).collect();

    // Determine focused agent (nearest to mouse)
    let focused_idx: Option<usize> = mouse_world.and_then(|mw| {
        let mut best: Option<(usize, f32)> = None;
        for (i, a) in episode.agents.iter().enumerate() {
            if a.energy <= 0.0 { continue; }
            let dx = a.pos.x - mw.x; let dy = a.pos.y - mw.y; let d2 = dx*dx + dy*dy;
            if let Some((_, b)) = best { if d2 < b { best = Some((i, d2)); } } else { best = Some((i, d2)); }
        }
        best.map(|(i, _)| i)
    });

    // agents
    for (idx, a) in episode.agents.iter().enumerate() {
        let (px, py) = world_to_screen(area, a.pos);
        let agent_r = ((AGENT_RADIUS / WORLD_W) * area.w).max(3.0);
        let sidx = *member_species.get(a.id.0).unwrap_or(&0usize);
        // draw alive vs dead differently
        if a.energy > 0.0 {
            let fill = species_color(sidx);
            draw_circle(px, py, agent_r, fill);
        } else {
            let fill = Color::new(0.25, 0.25, 0.25, 0.9);
            draw_circle(px, py, agent_r, fill);
        }
        draw_circle_lines(px, py, agent_r, 2.0, Color::new(0.2, 0.2, 0.2, 0.6));
        // heading line
        if a.energy > 0.0 {
            let dir = dir_from_theta(a.theta);
            let (hx, hy) = world_to_screen(area, Vec2 { x: a.pos.x + dir.x * 2.0, y: a.pos.y + dir.y * 2.0 });
            draw_line(px, py, hx, hy, 2.0, BLUE);
        }

        let mut any_food_sensed = false;
        if show_cones && a.energy > 0.0 {
            let dir = dir_from_theta(a.theta);
            for r in sensing::ray_directions(dir) {
                let food_t = sensing::nearest_food_along_ray(a.pos, r, &episode.food);
                match food_t {
                    Some(t) => {
                        any_food_sensed = true;
                        let sense_pt = Vec2 { x: a.pos.x + r.x * t, y: a.pos.y + r.y * t };
                        let (sx, sy) = world_to_screen(area, sense_pt);
                        // draw sensed segment in bright green up to the food point
                        draw_line(px, py, sx, sy, 2.0, Color::new(0.2, 1.0, 0.2, 0.9));
                        // mark the sensed point
                        draw_circle(sx, sy, 3.0, Color::new(0.2, 1.0, 0.2, 0.9));
                        // faint remainder to max range
                        let end = Vec2 { x: a.pos.x + r.x * VISION_RANGE, y: a.pos.y + r.y * VISION_RANGE };
                        let (x2, y2) = world_to_screen(area, end);
                        draw_line(sx, sy, x2, y2, 1.0, Color::new(0.0, 0.6, 1.0, 0.25));
                    }
                    None => {
                        let end = Vec2 { x: a.pos.x + r.x * VISION_RANGE, y: a.pos.y * 1.0 + r.y * VISION_RANGE };
                        let (x2, y2) = world_to_screen(area, end);
                        draw_line(px, py, x2, y2, 1.0, Color::new(0.0, 0.6, 1.0, 0.4));
                    }
                }
            }
        }

        // If any ray senses food, add a green highlight ring around the agent
        if any_food_sensed {
            draw_circle_lines(px, py, agent_r + 3.0, 2.0, Color::new(0.2, 1.0, 0.2, 0.9));
        }
        // Predation flash: red ring
        if a.predation_flash_steps > 0 {
            draw_circle_lines(px, py, agent_r + 5.0, 3.0, Color::new(1.0, 0.1, 0.1, 0.95));
        }

        // Overlays for the focused agent
        if Some(idx) == focused_idx && a.energy > 0.0 {
            if show_density_overlay {
                let bins = sensing::density_sectors(a.pos, a.theta, &snapshot, idx);
                let two_pi = std::f32::consts::PI * 2.0;
                let sector = two_pi / (DENSITY_SECTORS as f32);
                let base_len = 18.0_f32.max(agent_r + 4.0);
                for s in 0..DENSITY_SECTORS {
                    let v = bins[s].clamp(0.0, 1.0);
                    if v <= 0.0 { continue; }
                    // mid-angle of sector in local frame: 0 = right, +pi/2 = forward
                    let ang_local = -std::f32::consts::PI + sector * (s as f32 + 0.5);
                    // Convert local dir to world delta with scale
                    let c = a.theta.cos(); let snt = a.theta.sin();
                    let right_x = -snt; let right_y = c;
                    let fwd_x = c; let fwd_y = snt;
                    let dir_world_x = right_x * ang_local.cos() + fwd_x * ang_local.sin();
                    let dir_world_y = right_y * ang_local.cos() + fwd_y * ang_local.sin();
                    let len = base_len + v * 28.0;
                    let end_world = Vec2 { x: a.pos.x + dir_world_x * (len / area.w * WORLD_W), y: a.pos.y + dir_world_y * (len / area.h * WORLD_H) };
                    let (ex, ey) = world_to_screen(area, end_world);
                    draw_line(px, py, ex, ey, 2.0, Color::new(0.1, 1.0, 1.0, 0.8));
                }
            }
            if show_vector_overlay {
                // Helper to draw an arrow for a local vector
                let draw_local_arrow = |_label: &str, lx: f32, ly: f32, color: Color| {
                    let c = a.theta.cos(); let snt = a.theta.sin();
                    let right_x = -snt; let right_y = c;
                    let fwd_x = c; let fwd_y = snt;
                    let scale = 60.0; // pixels
                    let world_dx = (right_x * lx + fwd_x * ly) * (scale / area.w * WORLD_W);
                    let world_dy = (right_y * lx + fwd_y * ly) * (scale / area.h * WORLD_H);
                    let end = Vec2 { x: a.pos.x + world_dx, y: a.pos.y + world_dy };
                    let (ex, ey) = world_to_screen(area, end);
                    draw_line(px, py, ex, ey, 2.0, color);
                    // arrow head
                    let hx = ex + (px - ex) * 0.15 + (ey - py) * 0.12;
                    let hy = ey + (py - ey) * 0.15 - (ex - px) * 0.12;
                    draw_line(ex, ey, hx, hy, 2.0, color);
                    let hx2 = ex + (px - ex) * 0.15 - (ey - py) * 0.12;
                    let hy2 = ey + (py - ey) * 0.15 + (ex - px) * 0.12;
                    draw_line(ex, ey, hx2, hy2, 2.0, color);
                };
                // Current food vector (green)
                let (fx, fy) = sensing::nearest_food_vector_local(a.pos, a.theta, &episode.food);
                draw_local_arrow("food", fx, fy, Color::new(0.2, 1.0, 0.2, 0.95));
                // Current danger vector (red)
                let (dx, dy) = sensing::nearest_agent_vector_local(a.pos, a.theta, &snapshot, idx);
                draw_local_arrow("danger", dx, dy, Color::new(1.0, 0.2, 0.2, 0.9));
                // Memory vectors (yellow/orange)
                draw_local_arrow("last_food", a.last_food_mem.x, a.last_food_mem.y, Color::new(1.0, 0.9, 0.2, 0.95));
                draw_local_arrow("last_danger", a.last_danger_mem.x, a.last_danger_mem.y, Color::new(1.0, 0.6, 0.2, 0.95));
            }
        }
    }
}

fn draw_hud(area: Rect, state: &AppState, running: bool, fast_mode: bool, member_species: &[usize]) {
    // Sidebar panel to avoid overflow
    let padding = 12.0;
    let mut y = area.y + padding;
    let x = area.x + padding;
    // Reserve bottom portion for network panel
    let network_h = (area.h * 0.42).clamp(160.0, 380.0);
    let max_y = area.y + area.h - padding - network_h - 8.0;
    let font_size = 18.0;
    let total_eaten: usize = state.episode.agents.iter().map(|a| a.eaten).sum();
    let alive = state.episode.agents.iter().filter(|a| a.energy > 0.0).count();
    let (min_energy, max_energy, avg_energy) = if !state.episode.agents.is_empty() {
        let mut min_e = f32::INFINITY; let mut max_e = f32::NEG_INFINITY; let mut sum = 0.0;
        for a in &state.episode.agents { min_e = min_e.min(a.energy); max_e = max_e.max(a.energy); sum += a.energy; }
        (min_e, max_e, sum / state.episode.agents.len() as f32)
    } else { (0.0, 0.0, 0.0) };
    let corpses = state.episode.agents.iter().filter(|a| a.energy <= 0.0 && !a.consumed).count();
    let plants = state.episode.food.len();
    let species_count = state.speciator.get_species().len();
    let first_eat = state.episode.first_eat_step.map(|s| s.to_string()).unwrap_or("-".to_string());
    let mode = if !running { "Paused" } else if fast_mode { "Running (Fast)" } else { "Running (Normal)" };
    let lines = vec![
        format!("Generation: {}", state.generation),
        format!("Population: {}  Species: {}", state.population.len(), species_count),
        format!("Mode: {}", mode),
        format!("Best: {:.3}  Avg: {:.3}", state.last_best, state.last_avg),
        format!("Steps: {}  First eat: {}", state.episode.steps, first_eat),
        format!("Plants: {}  Corpses: {}", plants, corpses),
        format!("Alive: {}  Total eaten: {}", alive, total_eaten),
        format!("Energy min/avg/max: {:.0} / {:.0} / {:.0}", min_energy, avg_energy, max_energy),
        format!("Inputs: {}  Rays: {}  Range: {:.0}", INPUTS, VISION_RAYS, VISION_RANGE),
        format!("Move: turn={:.2} rad  speed={:.1}", MAX_TURN, MAX_SPEED),
        format!("Turn cost: {:.3}  Thrust coupling: {:.2}", TURN_COST, THRUST_TURN_COUPLING),
        format!("Food vec range: {:.0}", FOOD_VECTOR_MAX_RANGE),
        format!("Danger vec range: {:.0}", DANGER_VECTOR_MAX_RANGE),
        format!("Density: sectors={} radius={:.0}", DENSITY_SECTORS, DENSITY_RADIUS),
        format!("Corpse decay: {:.1}%/step  Digest (plant/meat): {}/{} steps", CORPSE_DECAY_RATE*100.0, DIGEST_STEPS_PLANT, DIGEST_STEPS_MEAT),
        format!("Eaten weight: {:.2}  Step weight: {:.3}  Expl/cell: {:.3}", EAT_WEIGHT, STEP_WEIGHT, EXPL_REWARD_PER_CELL),
        format!("Predation: {}  Scavenge: {}  Meat energy: {:.0}", PREDATION_ENABLED, SCAVENGE_ENABLED, MEAT_ENERGY),
        "Controls:".to_string(),
        "  [P] pause/resume   [F] fast/normal".to_string(),
        "  [R] reset episode  [V] toggle vision  [D] density overlay  [B] vector overlay".to_string(),
        "  Hover an agent in the world to see overlays".to_string(),
        "Species (last gen):".to_string(),
    ];
    let mut species = state.last_species.clone();
    species.sort_by(|a, b| b.best_fitness.partial_cmp(&a.best_fitness).unwrap_or(std::cmp::Ordering::Equal));
    // Panel background
    draw_rectangle(area.x, area.y, area.w, area.h, Color::new(0.08, 0.08, 0.10, 0.95));
    draw_rectangle_lines(area.x, area.y, area.w, area.h, 2.0, GRAY);
    for line in lines {
        if y > max_y { break; }
        draw_text_clamped(&line, x, y, font_size, WHITE, area.w - (x - area.x) - padding);
        y += font_size + 6.0;
    }
    // Species lines with color swatch
    for (rank, s) in species.into_iter().enumerate() {
        if y > max_y { break; }
        let color = species_color(rank);
        // swatch
        let sw_h = font_size * 0.8;
        let sw_w = sw_h * 1.4;
        draw_rectangle(x, y - sw_h + 2.0, sw_w, sw_h, color);
        draw_rectangle_lines(x, y - sw_h + 2.0, sw_w, sw_h, 1.0, BLACK);
        let text = format!(
            "  mem={} best={:.2} adj={:.2} stagn={} rep={} (rank #{:02})",
            s.members.len(), s.best_fitness, s.adjusted_fitness, s.stagnant_generations, s.representative, rank
        );
        draw_text(&text, x + sw_w + 6.0, y, font_size, WHITE);
        y += font_size + 6.0;
    }
    if y <= max_y {
        // Live predation summary
        let mut max_idx = 0usize;
        for &sidx in member_species { if sidx > max_idx { max_idx = sidx; } }
        let mut kills_per_species = vec![0usize; max_idx + 1];
        let mut preds_per_species = vec![0usize; max_idx + 1];
        for a in &state.episode.agents {
            if a.kills > 0 && a.id.0 < member_species.len() {
                let sidx = member_species[a.id.0];
                kills_per_species[sidx] += a.kills;
                preds_per_species[sidx] += 1;
            }
        }
        // Header
        draw_text("Predation (live):", x, y, font_size, WHITE);
        y += font_size + 6.0;
        for sidx in 0..kills_per_species.len() {
            if y > max_y { break; }
            if kills_per_species[sidx] == 0 { continue; }
            let color = species_color(sidx);
            let sw_h = font_size * 0.8; let sw_w = sw_h * 1.4;
            draw_rectangle(x, y - sw_h + 2.0, sw_w, sw_h, color);
            draw_rectangle_lines(x, y - sw_h + 2.0, sw_w, sw_h, 1.0, BLACK);
            let text = format!("  kills={} preds={}", kills_per_species[sidx], preds_per_species[sidx]);
            draw_text_clamped(&text, x + sw_w + 6.0, y, font_size, WHITE, area.w - (x + sw_w + 6.0 - area.x) - padding);
            y += font_size + 6.0;
        }
    }
    // Draw best-network panel at bottom
    let panel = Rect {
        x: area.x + 8.0,
        y: area.y + area.h - network_h + 8.0,
        w: area.w - 16.0,
        h: network_h - 16.0,
    };
    draw_rectangle(panel.x - 4.0, panel.y - 4.0, panel.w + 8.0, panel.h + 8.0, Color::new(0.05, 0.05, 0.07, 0.95));
    draw_rectangle_lines(panel.x - 4.0, panel.y - 4.0, panel.w + 8.0, panel.h + 8.0, 2.0, Color::new(0.25, 0.25, 0.3, 1.0));
    let title = if state.last_best.is_finite() && state.last_best > f32::NEG_INFINITY {
        format!("Best network (last gen {}, fit {:.2})", state.last_best_generation, state.last_best)
    } else {
        "Best network (pending)".to_string()
    };
    draw_text_clamped(&title, panel.x, panel.y - 8.0, 18.0, LIGHTGRAY, panel.w - 8.0);
    if let Some(genome) = state.last_best_genome.as_ref() {
        draw_network_panel(panel, genome);
    } else {
        let msg = "Evolves as episodes complete. Once a new best is found, its network will appear here.";
        draw_text_clamped(msg, panel.x, panel.y + panel.h * 0.5, 16.0, GRAY, panel.w - 8.0);
    }
    return;
}

fn draw_network_panel(area: Rect, genome: &Genome) {
    // Compute layout: inputs (left), hidden (middle), outputs (right)
    // Collect nodes by type and sort by id for stability
    let mut inputs: Vec<u32> = Vec::new();
    let mut hiddens: Vec<u32> = Vec::new();
    let mut outputs: Vec<u32> = Vec::new();
    for (id, node) in &genome.nodes {
        match node.node_type {
            NodeType::Input => inputs.push(*id),
            NodeType::Hidden => hiddens.push(*id),
            NodeType::Output => outputs.push(*id),
            NodeType::Bias => {} // not used in this setup
        }
    }
    inputs.sort_unstable();
    hiddens.sort_unstable();
    outputs.sort_unstable();

    // Positions
    let left_x = area.x + 40.0;
    let right_x = area.x + area.w - 40.0;
    let mid_x = (left_x + right_x) * 0.5;
    let top_y = area.y + 24.0;
    let bot_y = area.y + area.h - 24.0;

    let mut pos: std::collections::HashMap<u32, (f32, f32)> = std::collections::HashMap::new();
    let place_col = |ids: &Vec<u32>, x: f32, pos: &mut std::collections::HashMap<u32, (f32, f32)>| {
        let n = ids.len().max(1) as f32;
        for (i, id) in ids.iter().enumerate() {
            let t = if n <= 1.0 { 0.5 } else { i as f32 / (n - 1.0) };
            let y = top_y * (1.0 - t) + bot_y * t;
            pos.insert(*id, (x, y));
        }
    };
    place_col(&inputs, left_x, &mut pos);
    place_col(&hiddens, mid_x, &mut pos);
    place_col(&outputs, right_x, &mut pos);

    // Draw connections first
    for conn in &genome.connections {
        if !conn.enabled { continue; }
        if let (Some(&(x1, y1)), Some(&(x2, y2))) = (pos.get(&conn.in_node_id), pos.get(&conn.out_node_id)) {
            let w = (conn.weight.abs() * 2.0).clamp(1.0, 4.0);
            let col = if conn.weight >= 0.0 { Color::new(0.2, 0.9, 0.3, 0.85) } else { Color::new(0.95, 0.25, 0.25, 0.85) };
            draw_line(x1, y1, x2, y2, w, col);
        }
    }
    // Draw nodes on top
    let draw_nodes = |ids: &Vec<u32>, color: Color| {
        for id in ids {
            if let Some(&(x, y)) = pos.get(id) {
                draw_circle(x, y, 3.0, color);
                draw_circle_lines(x, y, 3.0, 1.5, BLACK);
            }
        }
    };
    draw_nodes(&inputs, Color::new(0.2, 0.6, 1.0, 1.0));
    draw_nodes(&hiddens, Color::new(0.8, 0.8, 0.85, 1.0));
    draw_nodes(&outputs, Color::new(1.0, 0.6, 0.2, 1.0));

    // Legends
    let legend_y = area.y + 16.0;
    let mut lx = area.x + 8.0;
    let legend = |lx: &mut f32, label: &str, col: Color| {
        draw_circle(*lx + 8.0, legend_y, 6.0, col);
        draw_circle_lines(*lx + 8.0, legend_y, 6.0, 1.0, BLACK);
        draw_text(label, *lx + 18.0, legend_y + 4.0, 14.0, LIGHTGRAY);
        *lx += 90.0;
    };
    legend(&mut lx, "Inputs", Color::new(0.2, 0.6, 1.0, 1.0));
    legend(&mut lx, "Hidden", Color::new(0.8, 0.8, 0.85, 1.0));
    legend(&mut lx, "Outputs", Color::new(1.0, 0.6, 0.2, 1.0));
}

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
    let mut state = AppState::new(25);
    let mut running = true;      // continuous evolution by default
    let mut fast_mode = false;   // start at normal speed
    let mut normal_step_timer = 0.0f32;          // accumulates frame time for normal stepping
    let normal_step_interval = 0.02f32;           // seconds per simulation step in normal mode
    let fast_steps_per_frame: usize = 1000;       // simulation steps per frame in fast mode

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
    let mouse_world = if mx >= world_area.x && mx <= world_area.x + world_area.w && my >= world_area.y && my <= world_area.y + world_area.h {
        Some(screen_to_world(world_area, mx, my))
    } else { None };

    draw_world(
        world_area,
        &state.episode,
        state.show_cones,
        &state.member_species,
        state.show_density_overlay,
        state.show_vector_overlay,
        mouse_world,
    );
    draw_hud(hud_area, &state, running, fast_mode, &state.member_species);

        next_frame().await
    }
}

fn draw_text_clamped(text: &str, x: f32, y: f32, font_size: f32, color: Color, max_width: f32) {
    let dims = measure_text(text, None, font_size as u16, 1.0);
    if dims.width <= max_width {
        draw_text(text, x, y, font_size, color);
        return;
    }
    // Estimate a cut with ellipsis
    let total_chars = text.chars().count().max(1) as f32;
    let avg_w = (dims.width / total_chars).max(1.0);
    let mut take = ((max_width - 10.0) / avg_w).floor().max(0.0) as usize;
    if take == 0 { return; }
    let mut s: String = text.chars().take(take).collect();
    s.push('…');
    let dims2 = measure_text(&s, None, font_size as u16, 1.0);
    if dims2.width > max_width && take > 1 {
        take = take.saturating_sub(2);
        s = text.chars().take(take).collect();
        s.push('…');
    }
    draw_text(&s, x, y, font_size, color);
}
