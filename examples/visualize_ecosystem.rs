use macroquad::prelude::*;
use neat::neat::{
    config::EvolutionConfig,
    evolution,
    genome::Genome,
    innovation_tracker::InnovationTracker,
    speciator::Speciator,
};
use neat::neat::species::Species;
use ::rand::Rng;

// World/agent constants (continuous space)
const WORLD_W: f32 = 500.0;
const WORLD_H: f32 = 500.0;
const FOOD_COUNT: usize = 40;
const FOOD_RADIUS: f32 = 1.2;
const AGENT_RADIUS: f32 = 1.5;
const INITIAL_ENERGY: f32 = 300.0;
const ENERGY_DRAIN_PER_STEP: f32 = 0.4;
const FOOD_ENERGY: f32 = 75.0;
const MAX_STEPS: usize = 500;

// Plant/food dynamics
const MAX_FOOD: usize = 150;                 // hard cap on number of plants
const FOOD_MIN_SEP: f32 = 2.5;               // minimum separation between plants
const FOOD_RESPAWN_PROB: f32 = 0.06;         // per-step probability to spawn one random plant
const FOOD_SPREAD_CHANCE: f32 = 0.02;        // per existing plant, chance to spawn a nearby offshoot
const FOOD_SPREAD_RADIUS: f32 = 15.0;        // max radius for offshoot from parent plant

// Vision cone parameters
const VISION_RAYS: usize = 11;           // number of rays within the cone
const VISION_ANGLE_DEG: f32 = 90.0;     // total cone angle
const VISION_RANGE: f32 = 200.0;         // world units

// Movement
const MAX_TURN: f32 = std::f32::consts::PI / 12.0; // radians per step at full turn
const MAX_SPEED: f32 = 2.5;                        // units per step at full thrust

// Reduce circling: scale thrust down when turning strongly
const THRUST_TURN_COUPLING: f32 = 1.0; // 0 = none, 1 = full: thrust*(1-|turn|)

// Predation/scavenging
const EAT_AGENT_RADIUS: f32 = AGENT_RADIUS + AGENT_RADIUS;
const MEAT_ENERGY: f32 = 60.0;           // energy gained by eating an agent (alive or dead)
const PREDATION_ENABLED: bool = true;     // eat live agents when close
const SCAVENGE_ENABLED: bool = true;      // eat dead agents when close

const INPUTS: usize = VISION_RAYS * 2 + 3; // per-ray [food, wall] + food_vector(x,y) + energy
const OUTPUTS: usize = 2; // turn, thrust

// Exploration and avoidance (mirrors headless)
const EXPL_CELL_SIZE: f32 = 10.0;
const EXPL_REWARD_PER_CELL: f32 = 0.02;
const AVOID_RADIUS: f32 = 3.0;
const AVOID_PENALTY_SCALE: f32 = 0.005;
const AVOID_CHECK_EVERY: usize = 2;
const EPISODES_PER_GEN: usize = 3; // average fitness over multiple randomized episodes

// Food-direction vector parameters
const FOOD_VECTOR_MAX_RANGE: f32 = 150.0; // how far we consider plants when building the vector

// Turn and fitness shaping
const TURN_COST: f32 = 0.05;     // energy drain per unit |turn|
const EAT_WEIGHT: f32 = 4.5;     // fitness weight for each item eaten (plants or agents)
const STEP_WEIGHT: f32 = 0.001;  // fitness weight per step survived

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

fn rand_pos<R: Rng>(rng: &mut R) -> Vec2 {
    Vec2::new(rng.random_range(0.0..WORLD_W), rng.random_range(0.0..WORLD_H))
}

fn can_place_food(existing: &[Vec2], p: Vec2) -> bool {
    let min_d2 = FOOD_MIN_SEP * FOOD_MIN_SEP;
    for f in existing {
        let dx = f.x - p.x; let dy = f.y - p.y;
        if dx*dx + dy*dy < min_d2 { return false; }
    }
    true
}

fn build_world<R: Rng>(rng: &mut R) -> Vec<Vec2> {
    let mut food = Vec::with_capacity(FOOD_COUNT);
    let mut attempts = 0;
    while food.len() < FOOD_COUNT && attempts < FOOD_COUNT * 50 {
        attempts += 1;
        let p = rand_pos(rng);
        if can_place_food(&food, p) { food.push(p); }
    }
    food
}

fn try_spawn_food_random<R: Rng>(food: &mut Vec<Vec2>, rng: &mut R) {
    if food.len() >= MAX_FOOD { return; }
    let p = rand_pos(rng);
    if can_place_food(food, p) { food.push(p); }
}

fn try_spawn_food_near<R: Rng>(food: &mut Vec<Vec2>, rng: &mut R, center: Vec2) {
    if food.len() >= MAX_FOOD { return; }
    let ang = rng.random_range(0.0..(std::f32::consts::PI * 2.0));
    let r = rng.random_range(0.5..FOOD_SPREAD_RADIUS);
    let p = Vec2 { x: (center.x + ang.cos() * r).clamp(0.0, WORLD_W), y: (center.y + ang.sin() * r).clamp(0.0, WORLD_H) };
    if can_place_food(food, p) { food.push(p); }
}

fn food_growth_step<R: Rng>(food: &mut Vec<Vec2>, rng: &mut R) {
    // Random spawn
    if rng.random_range(0.0..1.0) < FOOD_RESPAWN_PROB { try_spawn_food_random(food, rng); }
    // Local spread from existing plants (snapshot length to avoid cascading within the same step)
    let base_len = food.len();
    for i in 0..base_len {
        if food.len() >= MAX_FOOD { break; }
        if rng.random_range(0.0..1.0) < FOOD_SPREAD_CHANCE {
            let parent = food[i];
            try_spawn_food_near(food, rng, parent);
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
struct AgentId(usize);

#[derive(Clone, Copy, Debug)]
struct Agent {
    id: AgentId,
    pos: Vec2,
    theta: f32,
    energy: f32,
    eaten: usize,
    consumed: bool, // true if this agent's body has been eaten and removed from world
    kills: usize,                // number of agents eaten (live or dead) in this episode
    predation_flash_steps: u16,  // visual cue counter for recent predation
}

struct Episode {
    food: Vec<Vec2>,
    agents: Vec<Agent>,
    steps: usize,
}

fn dir_from_theta(theta: f32) -> Vec2 { Vec2 { x: theta.cos(), y: theta.sin() } }

fn ray_directions(dir: Vec2) -> Vec<Vec2> {
    let center_ang = dir.y.atan2(dir.x);
    let half = VISION_ANGLE_DEG.to_radians() * 0.5; let start = center_ang - half;
    let step = if VISION_RAYS > 1 { (2.0 * half) / (VISION_RAYS as f32 - 1.0) } else { 0.0 };
    (0..VISION_RAYS).map(|i| { let ang = start + step * (i as f32); Vec2 { x: ang.cos(), y: ang.sin() } }).collect()
}

fn ray_wall_distance(p: Vec2, dir: Vec2) -> f32 {
    let mut tmin = 0.0f32; let mut tmax = f32::INFINITY;
    if dir.x.abs() < 1e-6 { if p.x <= 0.0 || p.x >= WORLD_W { return 0.0; } } else {
        let inv = 1.0/dir.x; let mut t1 = (0.0 - p.x) * inv; let mut t2 = (WORLD_W - p.x) * inv; if t1>t2 { std::mem::swap(&mut t1, &mut t2); } tmin = tmin.max(t1); tmax = tmax.min(t2);
    }
    if dir.y.abs() < 1e-6 { if p.y <= 0.0 || p.y >= WORLD_H { return 0.0; } } else {
        let inv = 1.0/dir.y; let mut t1 = (0.0 - p.y) * inv; let mut t2 = (WORLD_H - p.y) * inv; if t1>t2 { std::mem::swap(&mut t1, &mut t2); } tmin = tmin.max(t1); tmax = tmax.min(t2);
    }
    if tmax < tmin { return 0.0; }
    if tmin > 0.0 { tmin } else { tmax.max(0.0) }
}

fn nearest_food_along_ray(p: Vec2, dir: Vec2, food: &[Vec2]) -> Option<f32> {
    let mut best: Option<f32> = None;
    for f in food {
        let op = Vec2 { x: f.x - p.x, y: f.y - p.y };
        let t = op.x * dir.x + op.y * dir.y;
        if t <= 0.0 || t > VISION_RANGE { continue; }
        let closest = Vec2 { x: p.x + dir.x * t, y: p.y + dir.y * t };
        let dx = f.x - closest.x; let dy = f.y - closest.y; let dist = (dx*dx + dy*dy).sqrt();
        if dist <= FOOD_RADIUS { match best { Some(b) if t >= b => {}, _ => best = Some(t) } }
    }
    best
}

fn nearest_food_vector_local(pos: Vec2, theta: f32, food: &[Vec2]) -> (f32, f32) {
    // Find nearest plant, build a direction vector in agent-local frame, attenuated by distance
    let mut best_d2 = f32::INFINITY;
    let mut best_v = Vec2 { x: 0.0, y: 0.0 };
    for f in food {
        let dx = f.x - pos.x; let dy = f.y - pos.y;
        let d2 = dx*dx + dy*dy;
        if d2 < best_d2 { best_d2 = d2; best_v = Vec2 { x: dx, y: dy }; }
    }
    if !best_d2.is_finite() || best_d2.is_infinite() || food.is_empty() { return (0.0, 0.0); }
    let d = best_d2.sqrt();
    let att = (1.0 - (d / FOOD_VECTOR_MAX_RANGE)).clamp(0.0, 1.0);
    if att <= 0.0 { return (0.0, 0.0); }
    // Rotate into agent frame: forward=(cos, sin), right=(-sin, cos)
    let c = theta.cos(); let s = theta.sin();
    let fwd_x = c; let fwd_y = s;
    let right_x = -s; let right_y = c;
    let dot_fwd = (best_v.x * fwd_x + best_v.y * fwd_y) / (d.max(1e-6));
    let dot_right = (best_v.x * right_x + best_v.y * right_y) / (d.max(1e-6));
    (dot_right * att, dot_fwd * att)
}

fn sample_inputs(pos: Vec2, theta: f32, food: &[Vec2], energy: f32) -> [f32; INPUTS] {
    let mut inputs = [0.0f32; INPUTS];
    let dir = dir_from_theta(theta);
    let rays = ray_directions(dir);
    let mut k = 0;
    for r in rays {
        let rdir = r.normalized();
        let food_t = nearest_food_along_ray(pos, rdir, food);
        let food_sig = food_t.map(|t| 1.0 - (t / VISION_RANGE)).unwrap_or(0.0);
        let wall_t = ray_wall_distance(pos, rdir);
        let wall_sig = if wall_t.is_finite() { (1.0 - (wall_t / VISION_RANGE)).clamp(0.0, 1.0) } else { 0.0 };
        inputs[k] = food_sig; k += 1; inputs[k] = wall_sig; k += 1;
    }
    let (fx, fy) = nearest_food_vector_local(pos, theta, food);
    inputs[k] = fx; k += 1; inputs[k] = fy; k += 1;
    inputs[k] = energy.clamp(0.0, 1.0);
    inputs
}

fn grid_index(p: Vec2) -> u32 {
    let nx = (WORLD_W / EXPL_CELL_SIZE).ceil() as u32;
    let ix = (p.x / EXPL_CELL_SIZE).floor().clamp(0.0, nx as f32 - 1.0) as u32;
    let iy = (p.y / EXPL_CELL_SIZE).floor().clamp(0.0, (WORLD_H / EXPL_CELL_SIZE).ceil() as f32 - 1.0) as u32;
    iy * nx + ix
}

fn eat_if_near(food: &mut Vec<Vec2>, pos: Vec2) -> bool {
    if food.is_empty() { return false; }
    if let Some((idx, _)) = food.iter().enumerate()
        .map(|(i, f)| (i, ((f.x - pos.x).powi(2) + (f.y - pos.y).powi(2)).sqrt()))
        .filter(|(_, d)| *d <= (FOOD_RADIUS + AGENT_RADIUS))
        .min_by(|a, b| a.1.total_cmp(&b.1)) {
        food.swap_remove(idx);
        true
    } else { false }
}

fn eval_population_single_episode(population: &[Genome]) -> Vec<f32> {
    let mut rng = ::rand::rng();
    let mut food = build_world(&mut rng);
    let mut agents: Vec<Agent> = population.iter().enumerate().map(|(i, _)| Agent {
        id: AgentId(i), pos: rand_pos(&mut rng), theta: -std::f32::consts::FRAC_PI_2, energy: INITIAL_ENERGY, eaten: 0, consumed: false, kills: 0, predation_flash_steps: 0,
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
            visited[i].insert(grid_index(a.pos));
            let energy_in = (a.energy / INITIAL_ENERGY).clamp(0.0, 1.0);
            let out = population[i].evaluate_slice(&sample_inputs(a.pos, a.theta, &food, energy_in));
            let turn = out.get(0).copied().unwrap_or(0.0).clamp(-1.0, 1.0);
            let thrust = out.get(1).copied().unwrap_or(0.0).clamp(0.0, 1.0);
            // Coupling: reduce thrust when turning strongly
            let thrust_eff = (thrust * (1.0 - THRUST_TURN_COUPLING * turn.abs())).clamp(0.0, 1.0);
            a.theta += turn * MAX_TURN;
            let dir = dir_from_theta(a.theta);
            let vel = dir.mul(thrust_eff * MAX_SPEED);
            a.pos = a.pos.add(vel).clamp_to_world();
            if eat_if_near(&mut food, a.pos) { a.energy = (a.energy + FOOD_ENERGY).min(INITIAL_ENERGY); a.eaten += 1; }
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
        }
        // Resolve predation after movement without aliasing borrows
        let mut claimed = vec![false; agents.len()];
        for i in 0..agents.len() {
            if let Some(j) = prey_targets[i] {
                if claimed[j] || agents[j].consumed { continue; }
                let alive_j = agents[j].energy > 0.0;
                if (alive_j && !PREDATION_ENABLED) || (!alive_j && !SCAVENGE_ENABLED) { continue; }
                agents[j].energy = 0.0;
                agents[j].consumed = true;
                agents[i].energy = (agents[i].energy + MEAT_ENERGY).min(INITIAL_ENERGY);
                agents[i].eaten += 1;
                agents[i].kills += 1;
                agents[i].predation_flash_steps = agents[i].predation_flash_steps.saturating_add(10);
                claimed[j] = true;
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
        food_growth_step(&mut food, &mut rng);
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
                pos: rand_pos(rng),
                theta: -std::f32::consts::FRAC_PI_2,
                energy: INITIAL_ENERGY,
                eaten: 0,
                consumed: false,
                kills: 0,
                predation_flash_steps: 0,
            });
        }
        Self { food: build_world(rng), agents, steps: 0 }
    }

    fn step<R: Rng>(&mut self, population: &[Genome], rng: &mut R) -> bool {
        // Live episode: continue until all agents are dead (ignore max steps and food exhaustion)
        if self.agents.iter().all(|a| a.energy <= 0.0) { return false; }
        // Snapshot for predation decisions
        let snapshot: Vec<(Vec2, bool, bool)> = self.agents.iter().map(|a| (a.pos, a.energy > 0.0, a.consumed)).collect();
        let mut prey_targets: Vec<Option<usize>> = vec![None; self.agents.len()];
        for (i, a) in self.agents.iter_mut().enumerate() {
            if a.energy <= 0.0 { continue; }
            let energy_in = (a.energy / INITIAL_ENERGY).clamp(0.0, 1.0);
            let out = population[a.id.0].evaluate_slice(&sample_inputs(a.pos, a.theta, &self.food, energy_in));
            let turn = out.get(0).copied().unwrap_or(0.0).clamp(-1.0, 1.0);
            let thrust = out.get(1).copied().unwrap_or(0.0).clamp(0.0, 1.0);
            // Coupling: reduce thrust when turning strongly
            let thrust_eff = (thrust * (1.0 - THRUST_TURN_COUPLING * turn.abs())).clamp(0.0, 1.0);
            a.theta += turn * MAX_TURN;
            let dir = dir_from_theta(a.theta);
            let vel = dir.mul(thrust_eff * MAX_SPEED);
            a.pos = a.pos.add(vel).clamp_to_world();
            // eat if close (sum of radii)
            if eat_if_near(&mut self.food, a.pos) {
                a.energy = (a.energy + FOOD_ENERGY).min(INITIAL_ENERGY);
                a.eaten += 1;
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
        }
        // Resolve predation after movement
        let mut claimed = vec![false; self.agents.len()];
        for i in 0..self.agents.len() {
            if let Some(j) = prey_targets[i] {
                if claimed[j] || self.agents[j].consumed { continue; }
                let alive_j = self.agents[j].energy > 0.0;
                if (alive_j && !PREDATION_ENABLED) || (!alive_j && !SCAVENGE_ENABLED) { continue; }
                self.agents[j].energy = 0.0;
                self.agents[j].consumed = true;
                self.agents[i].energy = (self.agents[i].energy + MEAT_ENERGY).min(INITIAL_ENERGY);
                self.agents[i].eaten += 1;
                self.agents[i].kills += 1;
                self.agents[i].predation_flash_steps = self.agents[i].predation_flash_steps.saturating_add(10);
                claimed[j] = true;
            }
        }
        // Plants grow/spread over time in the live world too
        food_growth_step(&mut self.food, rng);
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

fn draw_world(area: Rect, episode: &Episode, show_cones: bool, member_species: &[usize]) {
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
    // agents
    for a in &episode.agents {
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
            for r in ray_directions(dir) {
                let food_t = nearest_food_along_ray(a.pos, r, &episode.food);
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
                        let end = Vec2 { x: a.pos.x + r.x * VISION_RANGE, y: a.pos.y + r.y * VISION_RANGE };
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
    }
}

fn draw_hud(area: Rect, state: &AppState, running: bool, fast_mode: bool, member_species: &[usize]) {
    // Sidebar panel to avoid overflow
    let padding = 12.0;
    let mut y = area.y + padding;
    let x = area.x + padding;
    let max_y = area.y + area.h - padding;
    let font_size = 18.0;
    let total_eaten: usize = state.episode.agents.iter().map(|a| a.eaten).sum();
    let alive = state.episode.agents.iter().filter(|a| a.energy > 0.0).count();
    let avg_energy = if !state.episode.agents.is_empty() {
        state.episode.agents.iter().map(|a| a.energy).sum::<f32>() / state.episode.agents.len() as f32
    } else { 0.0 };
    let mode = if !running { "Paused" } else if fast_mode { "Running (Fast)" } else { "Running (Normal)" };
    let lines = vec![
        format!("Generation: {}", state.generation),
        format!("Population: {}", state.population.len()),
        format!("Mode: {}", mode),
        format!("Best: {:.3}", state.last_best),
        format!("Avg: {:.3}", state.last_avg),
        format!("Eaten total: {}", total_eaten),
        format!("Alive: {}", alive),
        format!("Avg energy: {:.1}", avg_energy),
        format!("Steps (live): {}", state.episode.steps),
        "Controls:".to_string(),
        "  [P] pause/resume   [F] fast/normal".to_string(),
        "  [R] reset episode  [V] toggle vision".to_string(),
        "Species (last gen):".to_string(),
    ];
    let mut species = state.last_species.clone();
    species.sort_by(|a, b| b.best_fitness.partial_cmp(&a.best_fitness).unwrap_or(std::cmp::Ordering::Equal));
    // Panel background
    draw_rectangle(area.x, area.y, area.w, area.h, Color::new(0.08, 0.08, 0.08, 0.9));
    draw_rectangle_lines(area.x, area.y, area.w, area.h, 2.0, GRAY);
    for line in lines {
        if y > max_y { break; }
        draw_text(&line, x, y, font_size, WHITE);
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
            draw_text(&text, x + sw_w + 6.0, y, font_size, WHITE);
            y += font_size + 6.0;
        }
    }
    return;
}

#[macroquad::main("NEAT Ecosystem Visualizer")]
async fn main() {
    let mut state = AppState::new(10);
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

    draw_world(world_area, &state.episode, state.show_cones, &state.member_species);
    draw_hud(hud_area, &state, running, fast_mode, &state.member_species);

        next_frame().await
    }
}
