use macroquad::prelude::*;
use neat::neat::{
    config::EvolutionConfig,
    evolution,
    genome::Genome,
    innovation_tracker::InnovationTracker,
    speciator::Speciator,
};
use ::rand::Rng;

// World/agent constants (continuous space)
const WORLD_W: f32 = 100.0;
const WORLD_H: f32 = 100.0;
const FOOD_COUNT: usize = 40;
const FOOD_RADIUS: f32 = 1.2;
const AGENT_RADIUS: f32 = 1.2;
const INITIAL_ENERGY: f32 = 300.0;
const ENERGY_DRAIN_PER_STEP: f32 = 1.0;
const FOOD_ENERGY: f32 = 75.0;
const MAX_STEPS: usize = 500;

// Vision cone parameters
const VISION_RAYS: usize = 5;           // number of rays within the cone
const VISION_ANGLE_DEG: f32 = 90.0;     // total cone angle
const VISION_RANGE: f32 = 20.0;         // world units

// Movement
const MAX_TURN: f32 = std::f32::consts::PI / 8.0; // radians per step at full turn
const MAX_SPEED: f32 = 2.5;                        // units per step at full thrust

const INPUTS: usize = VISION_RAYS * 2 + 1; // per-ray [food, wall] + energy
const OUTPUTS: usize = 2; // turn, thrust

// Exploration and avoidance (mirrors headless)
const EXPL_CELL_SIZE: f32 = 10.0;
const EXPL_REWARD_PER_CELL: f32 = 0.05;
const AVOID_RADIUS: f32 = 3.0;
const AVOID_PENALTY_SCALE: f32 = 0.005;
const AVOID_CHECK_EVERY: usize = 2;
const EPISODES_PER_GEN: usize = 3; // average fitness over multiple randomized episodes

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

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
struct AgentId(usize);

#[derive(Clone, Copy, Debug)]
struct Agent {
    id: AgentId,
    pos: Vec2,
    theta: f32,
    energy: f32,
    eaten: usize,
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

fn sample_cone_inputs(pos: Vec2, theta: f32, food: &[Vec2]) -> [f32; INPUTS] {
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
        .filter(|(_, d)| *d <= FOOD_RADIUS)
        .min_by(|a, b| a.1.total_cmp(&b.1)) {
        food.swap_remove(idx);
        true
    } else { false }
}

fn eval_population_single_episode(population: &[Genome]) -> Vec<f32> {
    let mut rng = ::rand::rng();
    let mut food = build_world(&mut rng);
    let mut agents: Vec<Agent> = population.iter().enumerate().map(|(i, _)| Agent {
        id: AgentId(i), pos: rand_pos(&mut rng), theta: -std::f32::consts::FRAC_PI_2, energy: INITIAL_ENERGY, eaten: 0,
    }).collect();
    let mut visited: Vec<std::collections::HashSet<u32>> = vec![std::collections::HashSet::new(); agents.len()];
    let mut avoid_penalty: Vec<f32> = vec![0.0; agents.len()];

    let mut steps = 0usize;
    while steps < MAX_STEPS {
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
            if eat_if_near(&mut food, a.pos) { a.energy = (a.energy + FOOD_ENERGY).min(INITIAL_ENERGY); a.eaten += 1; }
            a.energy -= ENERGY_DRAIN_PER_STEP + thrust * 0.2;
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
        steps += 1;
    }

    agents.iter().enumerate().map(|(i, a)| {
        let expl = visited[i].len() as f32 * EXPL_REWARD_PER_CELL;
        (a.eaten as f32) * 3.0 + (steps as f32) * 0.01 + expl - avoid_penalty[i]
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
            });
        }
        Self { food: build_world(rng), agents, steps: 0 }
    }

    fn step<R: Rng>(&mut self, population: &[Genome], _rng: &mut R) -> bool {
        // Live episode: continue until all agents are dead (ignore max steps and food exhaustion)
        if self.agents.iter().all(|a| a.energy <= 0.0) { return false; }
        for a in &mut self.agents {
            if a.energy <= 0.0 { continue; }
            let mut inputs = sample_cone_inputs(a.pos, a.theta, &self.food);
            inputs[INPUTS - 1] = (a.energy / INITIAL_ENERGY).clamp(0.0, 1.0);
            let out = population[a.id.0].evaluate_slice(&inputs);
            let turn = out.get(0).copied().unwrap_or(0.0).clamp(-1.0, 1.0);
            let thrust = out.get(1).copied().unwrap_or(0.0).clamp(0.0, 1.0);
            a.theta += turn * MAX_TURN;
            let dir = dir_from_theta(a.theta);
            let vel = dir.mul(thrust * MAX_SPEED);
            a.pos = a.pos.add(vel).clamp_to_world();
            // eat if close (sum of radii)
            if let Some((idx, _)) = self.food.iter().enumerate()
                .map(|(i, f)| (i, (f.x - a.pos.x).hypot(f.y - a.pos.y)))
                .filter(|(_, d)| *d <= (FOOD_RADIUS + AGENT_RADIUS))
                .min_by(|a, b| a.1.total_cmp(&b.1)) {
                self.food.swap_remove(idx);
                a.energy = (a.energy + FOOD_ENERGY).min(INITIAL_ENERGY);
                a.eaten += 1;
            }
            a.energy -= ENERGY_DRAIN_PER_STEP + thrust * 0.2;
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
}

impl AppState {
    fn new(pop_size: usize) -> Self {
        let num_inputs = INPUTS as u32;
        let num_outputs = OUTPUTS as u32;
        let mut rng = ::rand::rng();
        let mut innov = InnovationTracker::new();
        let speciator = Speciator::new(2.0);
        let cfg = EvolutionConfig { compatibility_threshold: 2.0, ..Default::default() };
        let population = Genome::create_initial_population(pop_size, num_inputs, num_outputs, &mut innov);
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
        self.generation += 1;
        let mut rng = ::rand::rng();
        self.episode = Episode::new(&mut rng, self.population.len());
    }
}

fn world_to_screen(area: Rect, p: Vec2) -> (f32, f32) {
    let sx = area.x + (p.x / WORLD_W) * area.w;
    let sy = area.y + (p.y / WORLD_H) * area.h;
    (sx, sy)
}

fn draw_world(area: Rect, episode: &Episode, show_cones: bool) {
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
    draw_circle(px, py, agent_r, SKYBLUE);
    draw_circle_lines(px, py, agent_r, 2.0, Color::new(0.2, 0.6, 1.0, 0.7));
        // heading line
        let dir = dir_from_theta(a.theta);
        let (hx, hy) = world_to_screen(area, Vec2 { x: a.pos.x + dir.x * 2.0, y: a.pos.y + dir.y * 2.0 });
        draw_line(px, py, hx, hy, 2.0, BLUE);
        if show_cones {
            for r in ray_directions(dir) {
                let end = Vec2 { x: a.pos.x + r.x * VISION_RANGE, y: a.pos.y + r.y * VISION_RANGE };
                let (x2, y2) = world_to_screen(area, end);
                draw_line(px, py, x2, y2, 1.0, Color::new(0.0, 0.6, 1.0, 0.4));
            }
        }
    }
}

fn draw_hud(area: Rect, state: &AppState, running: bool, fast_mode: bool) {
    // Sidebar panel to avoid overflow
    let padding = 12.0;
    let mut y = area.y + padding;
    let x = area.x + padding;
    let max_y = area.y + area.h - padding;
    let font_size = 20.0;
    let total_eaten: usize = state.episode.agents.iter().map(|a| a.eaten).sum();
    let alive = state.episode.agents.iter().filter(|a| a.energy > 0.0).count();
    let avg_energy = if !state.episode.agents.is_empty() {
        state.episode.agents.iter().map(|a| a.energy).sum::<f32>() / state.episode.agents.len() as f32
    } else { 0.0 };
    let mode = if !running { "Paused" } else if fast_mode { "Running (Fast)" } else { "Running (Normal)" };
    let lines = [
        format!("Generation: {}", state.generation),
        format!("Population: {}", state.population.len()),
        format!("Mode: {}", mode),
        format!("Best: {:.3}", state.last_best),
        format!("Avg: {:.3}", state.last_avg),
        format!("Eaten total: {}", total_eaten),
        format!("Alive: {}", alive),
        format!("Avg energy: {:.1}", avg_energy),
        format!("Steps (live): {}", state.episode.steps),
        "Controls: [P] pause/resume  [F] fast/normal  [R] reset episode  [V] toggle vision".to_string(),
    ];
    // Panel background
    draw_rectangle(area.x, area.y, area.w, area.h, Color::new(0.08, 0.08, 0.08, 0.9));
    draw_rectangle_lines(area.x, area.y, area.w, area.h, 2.0, GRAY);
    for line in lines {
        if y > max_y { break; }
        draw_text(&line, x, y, font_size, WHITE);
        y += font_size + 6.0;
    }
}

#[macroquad::main("NEAT Ecosystem Visualizer")]
async fn main() {
    let mut state = AppState::new(50);
    let mut running = true;      // continuous evolution by default
    let mut fast_mode = false;   // start at normal speed
    let mut normal_step_timer = 0.0f32;          // accumulates frame time for normal stepping
    let normal_step_interval = 0.005f32;           // seconds per simulation step in normal mode
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

    draw_world(world_area, &state.episode, state.show_cones);
    draw_hud(hud_area, &state, running, fast_mode);

        next_frame().await
    }
}
