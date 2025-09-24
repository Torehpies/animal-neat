use super::params::{
    WORLD_W, WORLD_H,
    FOOD_COUNT, FOOD_MIN_SEP, MAX_FOOD,
    FOOD_RESPAWN_PROB, FOOD_SPREAD_CHANCE, FOOD_SPREAD_RADIUS,
    FOOD_RADIUS, AGENT_RADIUS,
    BIOME_X_SPLITS, BIOME_RESPAWN_MULT, BIOME_SPREAD_MULT,
    SEASONAL_ENABLED, SEASONAL_PERIOD_STEPS, SEASONAL_AMPLITUDE, BIOME_SEASON_PHASE,
};
use super::Vec2;
use ::rand::Rng;

pub fn rand_pos<R: Rng>(rng: &mut R) -> Vec2 {
    Vec2 { x: rng.random_range(0.0..WORLD_W), y: rng.random_range(0.0..WORLD_H) }
}

fn can_place_food(existing: &[Vec2], p: Vec2) -> bool {
    let min_d2 = FOOD_MIN_SEP * FOOD_MIN_SEP;
    for f in existing {
        let dx = f.x - p.x; let dy = f.y - p.y;
        if dx*dx + dy*dy < min_d2 { return false; }
    }
    true
}

pub fn build_world<R: Rng>(rng: &mut R) -> Vec<Vec2> {
    let mut food = Vec::with_capacity(FOOD_COUNT);
    let mut attempts = 0;
    while food.len() < FOOD_COUNT && attempts < FOOD_COUNT * 50 {
        attempts += 1;
        let p = rand_pos(rng);
        if can_place_food(&food, p) { food.push(p); }
    }
    food
}

fn biome_index_for_x(x: f32) -> usize {
    let nx = x / WORLD_W;
    if nx < BIOME_X_SPLITS[0] { 0 }
    else if nx < BIOME_X_SPLITS[1] { 1 }
    else { 2 }
}

// Removed unused try_spawn_food_random (random spawns handled in food_growth_step)

pub fn try_spawn_food_near<R: Rng>(food: &mut Vec<Vec2>, rng: &mut R, center: Vec2) {
    if food.len() >= MAX_FOOD { return; }
    let ang = rng.random_range(0.0..(std::f32::consts::PI * 2.0));
    let r = rng.random_range(0.5..FOOD_SPREAD_RADIUS);
    let p = Vec2 { x: (center.x + ang.cos() * r).clamp(0.0, WORLD_W), y: (center.y + ang.sin() * r).clamp(0.0, WORLD_H) };
    if can_place_food(food, p) { food.push(p); }
}

pub fn food_growth_step<R: Rng>(food: &mut Vec<Vec2>, rng: &mut R) {
    // Seasonal factor function: f(biome) = 1 + A * sin(2π t/T + phase)
    let (t, season_amp) = if SEASONAL_ENABLED { (crate_current_step(), SEASONAL_AMPLITUDE) } else { (0usize, 0.0) };
    let season_factor = |biome: usize| -> f32 {
        if season_amp <= 0.0 || SEASONAL_PERIOD_STEPS == 0 { return 1.0; }
        let phase = BIOME_SEASON_PHASE[biome];
        let x = (t as f32) * std::f32::consts::TAU / (SEASONAL_PERIOD_STEPS as f32) + phase;
        (1.0 + season_amp * x.sin()).max(0.0)
    };

    // Random respawn attempt (biome-scaled)
    if food.len() < MAX_FOOD {
        let p = rand_pos(rng);
        let biome = biome_index_for_x(p.x);
        let prob = FOOD_RESPAWN_PROB * BIOME_RESPAWN_MULT[biome] * season_factor(biome);
        if rng.random_range(0.0..1.0) < prob { if can_place_food(food, p) { food.push(p); } }
    }
    // Spread from existing foods (biome-scaled)
    let base_len = food.len();
    for i in 0..base_len {
        if food.len() >= MAX_FOOD { break; }
        let parent = food[i];
        let biome = biome_index_for_x(parent.x);
        let prob = FOOD_SPREAD_CHANCE * BIOME_SPREAD_MULT[biome] * season_factor(biome);
        if rng.random_range(0.0..1.0) < prob {
            try_spawn_food_near(food, rng, parent);
        }
    }
}

pub fn eat_if_near(food: &mut Vec<Vec2>, pos: Vec2) -> bool {
    if food.is_empty() { return false; }
    if let Some((idx, _)) = food.iter().enumerate()
        .map(|(i, f)| (i, ((f.x - pos.x).powi(2) + (f.y - pos.y).powi(2)).sqrt()))
        .filter(|(_, d)| *d <= (FOOD_RADIUS + AGENT_RADIUS))
        .min_by(|a, b| a.1.total_cmp(&b.1)) {
        food.swap_remove(idx);
        true
    } else { false }
}

// Continuous collision: did the path from p0 to p1 pass within eat radius of any food?
pub fn eat_along_path(food: &mut Vec<Vec2>, p0: Vec2, p1: Vec2) -> bool {
    if food.is_empty() { return false; }
    let (vx, vy) = (p1.x - p0.x, p1.y - p0.y);
    let v_len2 = vx*vx + vy*vy;
    let eat_r = FOOD_RADIUS + AGENT_RADIUS;
    let eat_r2 = eat_r * eat_r;
    let mut best_i: Option<usize> = None;
    let mut best_t: f32 = f32::INFINITY;
    for (i, f) in food.iter().enumerate() {
        // Project (f - p0) onto v to clamp closest point on segment
        let wx = f.x - p0.x; let wy = f.y - p0.y;
        let mut t = if v_len2 > 0.0 { (wx*vx + wy*vy) / v_len2 } else { 0.0 };
        if t < 0.0 { t = 0.0; } else if t > 1.0 { t = 1.0; }
        let cx = p0.x + vx * t; let cy = p0.y + vy * t;
        let dx = f.x - cx; let dy = f.y - cy; let d2 = dx*dx + dy*dy;
        if d2 <= eat_r2 {
            // Prefer the earliest along the path
            if t < best_t { best_t = t; best_i = Some(i); }
        }
    }
    if let Some(i) = best_i { food.swap_remove(i); true } else { false }
}

// Hook to provide current global step for seasons; the visualizer sets this via a thread-local.
fn crate_current_step() -> usize {
    // Fallback: if not set, 0
    CURRENT_STEP.with(|c| c.get())
}

thread_local! {
    static CURRENT_STEP: std::cell::Cell<usize> = std::cell::Cell::new(0);
}

// Public setter used by visualize_ecosystem.rs to update world step before growth
pub fn set_current_step(step: usize) { CURRENT_STEP.with(|c| c.set(step)); }
