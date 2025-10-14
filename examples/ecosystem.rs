use neat::neat::{
    config::EvolutionConfig,
    evolution,
    genome::Genome,
    innovation_tracker::InnovationTracker,
    speciator::Speciator,
};
use rand::Rng;

// Multi-agent continuous 2D ecosystem simulation (no grid).
// Many agents (one per genome) compete to gather food before running out of energy.
// Each agent has a position and orientation. Vision is a forward-facing cone with rays; inputs report
// per-ray food and wall proximity plus the agent's normalized energy.
// Outputs (2): [turn, thrust]. Turn in [-1,1] scaled to MAX_TURN per step. Thrust in [0,1] scaled to MAX_SPEED.

const WORLD_W: f32 = 200.0;
const WORLD_H: f32 = 200.0;
const FOOD_COUNT: usize = 200;
const FOOD_RADIUS: f32 = 1.0;
const AGENT_RADIUS: f32 = 1.2;
const INITIAL_ENERGY: f32 = 100.0;
const ENERGY_DRAIN_PER_STEP: f32 = 1.0;
const FOOD_ENERGY: f32 = 20.0;
const MAX_STEPS: usize = 300;

// Vision cone parameters
const VISION_RAYS: usize = 5;           // number of rays within the cone
const VISION_ANGLE_DEG: f32 = 90.0;     // total cone angle
const VISION_RANGE: f32 = 20.0;         // world units

// Movement
const MAX_TURN: f32 = std::f32::consts::PI / 8.0; // radians per step at full turn
const MAX_SPEED: f32 = 2.5;                        // units per step at full thrust

const INPUTS: usize = VISION_RAYS * 2 + 1; // per-ray [food, wall] + energy
const OUTPUTS: usize = 2; // turn, thrust

// Exploration and avoidance
const EXPL_CELL_SIZE: f32 = 10.0;                 // world units per exploration cell
const EXPL_REWARD_PER_CELL: f32 = 0.05;           // reward per unique cell visited
const AVOID_RADIUS: f32 = 3.0;                    // penalty zone radius
const AVOID_PENALTY_SCALE: f32 = 0.005;           // per-step penalty scaled by proximity
const AVOID_CHECK_EVERY: usize = 2;               // compute avoidance penalty every N steps to save time
const EPISODES_PER_GEN: usize = 3;                // average fitness over multiple episodes for stability

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

fn build_world<R: Rng>(rng: &mut R) -> Vec<Vec2> {
    let mut food = Vec::with_capacity(FOOD_COUNT);
    while food.len() < FOOD_COUNT {
        food.push(rand_pos(rng));
    }
    food
}

#[derive(Clone, Copy, Debug)]
struct Agent {
    pos: Vec2,
    theta: f32,     // facing angle radians
    energy: f32,
    eaten: usize,
}

fn dir_from_theta(theta: f32) -> Vec2 { Vec2::new(theta.cos(), theta.sin()) }

fn ray_directions(dir: Vec2) -> Vec<Vec2> {
    // Generate VISION_RAYS directions spanning the cone centered at dir
    let center_ang = dir.y.atan2(dir.x); // Note: atan2(y, x)
    let half = VISION_ANGLE_DEG.to_radians() * 0.5;
    let start = center_ang - half;
    let step = if VISION_RAYS > 1 { (2.0 * half) / (VISION_RAYS as f32 - 1.0) } else { 0.0 };
    (0..VISION_RAYS).map(|i| {
        let ang = start + step * (i as f32);
        Vec2::new(ang.cos(), ang.sin())
    }).collect()
}
fn ray_wall_distance(p: Vec2, dir: Vec2) -> f32 {
    // Slab method for axis-aligned box [0..WORLD_W] x [0..WORLD_H]
    let mut tmin = 0.0f32;
    let mut tmax = f32::INFINITY;
    // X slab
    if dir.x.abs() < 1e-6 {
        if p.x <= 0.0 || p.x >= WORLD_W { return 0.0; }
    } else {
        let inv = 1.0 / dir.x;
        let mut t1 = (0.0 - p.x) * inv;
        let mut t2 = (WORLD_W - p.x) * inv;
        if t1 > t2 { std::mem::swap(&mut t1, &mut t2); }
        tmin = tmin.max(t1);
        tmax = tmax.min(t2);
    }
    // Y slab
    if dir.y.abs() < 1e-6 {
        if p.y <= 0.0 || p.y >= WORLD_H { return 0.0; }
    } else {
        let inv = 1.0 / dir.y;
        let mut t1 = (0.0 - p.y) * inv;
        let mut t2 = (WORLD_H - p.y) * inv;
        if t1 > t2 { std::mem::swap(&mut t1, &mut t2); }
        tmin = tmin.max(t1);
        tmax = tmax.min(t2);
    }
    if tmax < tmin { return 0.0; }
    let t_hit = if tmin > 0.0 { tmin } else { tmax.max(0.0) };
    t_hit
}

fn nearest_food_along_ray(p: Vec2, dir: Vec2, food: &[Vec2]) -> Option<f32> {
    let mut best: Option<f32> = None;
    for f in food {
        let op = Vec2::new(f.x - p.x, f.y - p.y);
        let t = op.dot(dir);
        if t <= 0.0 || t > VISION_RANGE { continue; }
        let closest = Vec2::new(p.x + dir.x * t, p.y + dir.y * t);
        let dist = Vec2::new(f.x - closest.x, f.y - closest.y).length();
        if dist <= FOOD_RADIUS {
            match best { Some(b) if t >= b => {}, _ => best = Some(t) }
        }
    }
    best
}

fn sample_cone_inputs(pos: Vec2, facing: f32, food: &[Vec2]) -> [f32; INPUTS] {
    let mut inputs = [0.0f32; INPUTS];
    let dir = dir_from_theta(facing);
    let rays = ray_directions(dir);
    let mut k = 0;
    for r in rays {
        let rdir = r.normalized();
        // Food distance along ray
        let food_t = nearest_food_along_ray(pos, rdir, food);
        let food_sig = food_t.map(|t| 1.0 - (t / VISION_RANGE)).unwrap_or(0.0);
        // Wall distance along ray
        let wall_t = ray_wall_distance(pos, rdir);
        let wall_sig = if wall_t.is_finite() { (1.0 - (wall_t / VISION_RANGE)).clamp(0.0, 1.0) } else { 0.0 };
        inputs[k] = food_sig; k += 1;
        inputs[k] = wall_sig; k += 1;
    }
    inputs
}

fn eat_if_near(food: &mut Vec<Vec2>, pos: Vec2) -> bool {
    if food.is_empty() { return false; }
    let eat_dist = FOOD_RADIUS + AGENT_RADIUS;
    if let Some((idx, _)) = food.iter().enumerate()
        .map(|(i, f)| (i, Vec2::new(f.x - pos.x, f.y - pos.y).length()))
        .filter(|(_, d)| *d <= eat_dist)
        .min_by(|a, b| a.1.total_cmp(&b.1)) {
        food.swap_remove(idx);
        true
    } else { false }
}

fn grid_index(p: Vec2) -> u32 {
    let nx = (WORLD_W / EXPL_CELL_SIZE).ceil() as u32;
    let ix = (p.x / EXPL_CELL_SIZE).floor().clamp(0.0, nx as f32 - 1.0) as u32;
    let iy = (p.y / EXPL_CELL_SIZE).floor().clamp(0.0, (WORLD_H / EXPL_CELL_SIZE).ceil() as f32 - 1.0) as u32;
    iy * nx + ix
}

fn eval_population_single_episode(population: &[Genome]) -> Vec<f32> {
    let mut rng = rand::rng();
    let mut food = build_world(&mut rng);
    // Spawn one agent per genome
    let mut agents: Vec<Agent> = population.iter().map(|_| Agent {
        pos: rand_pos(&mut rng),
        theta: -std::f32::consts::FRAC_PI_2, // facing up initially
        energy: INITIAL_ENERGY,
        eaten: 0,
    }).collect();

    let mut visited: Vec<std::collections::HashSet<u32>> = vec![std::collections::HashSet::new(); agents.len()];
    let mut avoid_penalty: Vec<f32> = vec![0.0; agents.len()];

    let mut step = 0usize;
    while step < MAX_STEPS {
        if food.is_empty() || agents.iter().all(|a| a.energy <= 0.0) { break; }
        for (i, a) in agents.iter_mut().enumerate() {
            if a.energy <= 0.0 { continue; }
            visited[i].insert(grid_index(a.pos));
            let mut inputs = sample_cone_inputs(a.pos, a.theta, &food);
            inputs[INPUTS - 1] = (a.energy / INITIAL_ENERGY).clamp(0.0, 1.0);
            let out = population[i].evaluate_slice(&inputs);
            let turn = out.get(0).copied().unwrap_or(0.0).clamp(-1.0, 1.0);
            let thrust = out.get(1).copied().unwrap_or(0.0).clamp(0.0, 1.0);
            a.theta += turn * MAX_TURN;
            let dir = dir_from_theta(a.theta);
            let vel = dir.mul(thrust * MAX_SPEED);
            a.pos = a.pos.add(vel).clamp_to_world();
            if eat_if_near(&mut food, a.pos) {
                a.energy = (a.energy + FOOD_ENERGY).min(INITIAL_ENERGY);
                a.eaten += 1;
            }
            a.energy -= ENERGY_DRAIN_PER_STEP + thrust * 0.2; // moving costs a bit extra
        }

        // Avoidance penalty (every few steps)
        if step % AVOID_CHECK_EVERY == 0 {
            for i in 0..agents.len() {
                if agents[i].energy <= 0.0 { continue; }
                let pi = agents[i].pos;
                let mut pen = 0.0f32;
                for j in 0..agents.len() {
                    if i == j || agents[j].energy <= 0.0 { continue; }
                    let pj = agents[j].pos;
                    let dx = pj.x - pi.x; let dy = pj.y - pi.y; let d2 = dx*dx + dy*dy;
                    let r2 = AVOID_RADIUS * AVOID_RADIUS;
                    if d2 < r2 {
                        let d = d2.sqrt();
                        let m = (AVOID_RADIUS - d) / AVOID_RADIUS; // 0..1
                        pen += m * AVOID_PENALTY_SCALE;
                    }
                }
                avoid_penalty[i] += pen;
            }
        }
        step += 1;
    }

    agents.iter().enumerate().map(|(i, a)| {
        let expl = visited[i].len() as f32 * EXPL_REWARD_PER_CELL;
        (a.eaten as f32) * 3.0 + (step as f32) * 0.01 + expl - avoid_penalty[i]
    }).collect()
}

fn eval_population_multi_agent(population: &[Genome]) -> Vec<f32> {
    // Average across multiple randomized episodes
    let mut acc = vec![0.0f32; population.len()];
    for _ in 0..EPISODES_PER_GEN {
        let mut scores = eval_population_single_episode(population);
        for (i, s) in scores.drain(..).enumerate() { acc[i] += s; }
    }
    for v in &mut acc { *v /= EPISODES_PER_GEN as f32; }
    acc
}

fn main() {
    let num_inputs = INPUTS as u32;
    let num_outputs = OUTPUTS as u32;
    let pop_size = 150usize;
    let generations = 200usize;
    let target = 20.0; // arbitrary target; tweak as desired

    let mut innov = InnovationTracker::new();
    let mut speciator = Speciator::new(2.0);
    let cfg = EvolutionConfig { compatibility_threshold: 2.0, ..Default::default() };

    let mut population = Genome::create_initial_population(pop_size, num_inputs, num_outputs, &mut innov);

    for step in 0..generations {
        // Score entire population together, averaged across episodes
        let fitness_scores: Vec<f32> = eval_population_multi_agent(&population);

        let best = fitness_scores.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        let avg = fitness_scores.iter().sum::<f32>() / fitness_scores.len() as f32;
        println!("Gen {}: best={:.3}, avg={:.3}", step, best, avg);

        if best >= target {
            println!("Target reached at generation {}", step);
            break;
        }

        // Evolve next generation
        population = evolution::evolution(
            population,
            fitness_scores,
            &mut speciator,
            &mut innov,
            &cfg,
        );
    }
}
