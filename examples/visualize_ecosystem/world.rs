use super::params::{
    WORLD_W, WORLD_H,
    FOOD_MIN_SEP, MAX_FOOD,
    FOOD_RESPAWN_PROB, FOOD_SPREAD_CHANCE, FOOD_SPREAD_RADIUS,
    FOOD_RADIUS,
    BIOME_X_SPLITS, BIOME_RESPAWN_MULT, BIOME_SPREAD_MULT,
    SEASONAL_ENABLED, SEASONAL_PERIOD_STEPS, SEASONAL_AMPLITUDE, BIOME_SEASON_PHASE,
    PLANT_DECAY_BASE_PER_TICK, PLANT_DECAY_SEASON_EXP, PLANT_DECAY_MAX_STEPS_PER_TICK,
    FOOD_RESPAWN_SEASON_EXP, FOOD_RESPAWN_CANDIDATE_SAMPLES, FOOD_SPREAD_SEASON_EXP,
    PLANT_LIFETIME_MIN_STEPS, PLANT_LIFETIME_MAX_STEPS,
};
use ::rand::Rng;
use macroquad::prelude::Vec2;
use crate::body::{Body, Plant};
use std::cell::Cell;

// Runtime config storage (thread-local)
thread_local! {
    static RUNTIME_WORLD_W: Cell<f32> = Cell::new(WORLD_W);
    static RUNTIME_WORLD_H: Cell<f32> = Cell::new(WORLD_H);
    static RUNTIME_MAX_FOOD: Cell<usize> = Cell::new(MAX_FOOD);
    static RUNTIME_FOOD_RESPAWN: Cell<f32> = Cell::new(FOOD_RESPAWN_PROB);
}

// Getters for runtime config (fallback to constants if not set)
fn world_w() -> f32 { RUNTIME_WORLD_W.with(|c| c.get()) }
fn world_h() -> f32 { RUNTIME_WORLD_H.with(|c| c.get()) }
fn max_food() -> usize { RUNTIME_MAX_FOOD.with(|c| c.get()) }
fn food_respawn_prob() -> f32 { RUNTIME_FOOD_RESPAWN.with(|c| c.get()) }

// Setter for runtime config (called from main before simulation starts)
pub fn set_runtime_config(world_width: f32, world_height: f32, max_food_count: usize, food_respawn_rate: f32) {
    RUNTIME_WORLD_W.with(|c| c.set(world_width));
    RUNTIME_WORLD_H.with(|c| c.set(world_height));
    RUNTIME_MAX_FOOD.with(|c| c.set(max_food_count));
    RUNTIME_FOOD_RESPAWN.with(|c| c.set(food_respawn_rate));
}

// Public getters for use by other modules (e.g., UI)
pub fn get_world_w() -> f32 { world_w() }
pub fn get_world_h() -> f32 { world_h() }

pub fn rand_pos<R: Rng>(rng: &mut R) -> Vec2 {
    Vec2 { x: rng.random_range(0.0..world_w()), y: rng.random_range(0.0..world_h()) }
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
    let target_food = max_food();
    let mut food = Vec::with_capacity(target_food);
    let mut attempts = 0;
    while food.len() < target_food && attempts < target_food * 50 {
        attempts += 1;
        let p = rand_pos(rng);
        if can_place_food(&food, p) { food.push(p); }
    }
    food
}

fn biome_index_for_x(x: f32) -> usize {
    let nx = x / world_w();
    if nx < BIOME_X_SPLITS[0] { 0 }
    else if nx < BIOME_X_SPLITS[1] { 1 }
    else { 2 }
}

// Removed unused try_spawn_food_random (random spawns handled in food_growth_step)

pub fn try_spawn_food_near<R: Rng>(food: &mut Vec<Vec2>, rng: &mut R, center: Vec2) {
    if food.len() >= max_food() { return; }
    let ang = rng.random_range(0.0..(std::f32::consts::PI * 2.0));
    let r = rng.random_range(0.5..FOOD_SPREAD_RADIUS);
    let p = Vec2 { x: (center.x + ang.cos() * r).clamp(0.0, world_w()), y: (center.y + ang.sin() * r).clamp(0.0, world_h()) };
    if can_place_food(food, p) { food.push(p); }
}

fn seasonal_factor_for_biome(t: usize, biome: usize) -> f32 {
    if !SEASONAL_ENABLED || SEASONAL_PERIOD_STEPS == 0 || SEASONAL_AMPLITUDE == 0.0 { return 1.0; }
    let phase = BIOME_SEASON_PHASE[biome];
    let x = (t as f32) * std::f32::consts::TAU / (SEASONAL_PERIOD_STEPS as f32) + phase;
    (1.0 + SEASONAL_AMPLITUDE * x.sin()).max(0.0)
}

fn sample_lifetime_for_pos<R: Rng>(rng: &mut R, _pos: Vec2) -> usize {
    // Base lifetime is drawn from the configured range; per-step aging will be season-adjusted dynamically.
    rng.random_range(PLANT_LIFETIME_MIN_STEPS as i32..=PLANT_LIFETIME_MAX_STEPS as i32) as usize
}

pub fn init_food_lifetimes<R: Rng>(food: &Vec<Vec2>, rng: &mut R) -> Vec<usize> {
    food.iter().map(|&p| sample_lifetime_for_pos(rng, p)).collect()
}

pub fn food_growth_and_aging_step<R: Rng>(food: &mut Vec<Vec2>, food_life: &mut Vec<usize>, rng: &mut R) {
    debug_assert_eq!(food.len(), food_life.len());
    // Age and remove expired, with season- and biome-dependent decay speed.
    let mut i = 0usize;
    while i < food.len() {
        if food_life[i] == 0 { food_life[i] = 1; }
        // Compute decay rate for this plant based on current season in its biome.
        let pos = food[i];
        let biome = biome_index_for_x(pos.x);
        let t = crate_current_step();
        let sf = seasonal_factor_for_biome(t, biome).max(0.0001);
        // Season-adjusted decay: base / sf^exp, then stochastically rounded to an integer decrement
        let decay = (PLANT_DECAY_BASE_PER_TICK / sf.powf(PLANT_DECAY_SEASON_EXP))
            .clamp(0.0, PLANT_DECAY_MAX_STEPS_PER_TICK);
        // Convert to integer decrement via stochastic rounding so we can exceed 2 when very bad
        let base = decay.floor() as usize;
        let frac = (decay - base as f32).max(0.0);
        let extra = if rng.random_range(0.0..1.0) < frac { 1 } else { 0 };
        let dec_steps: usize = base + extra;
        if dec_steps > 0 { food_life[i] = food_life[i].saturating_sub(dec_steps); }
        if food_life[i] == 0 {
            food.swap_remove(i);
            food_life.swap_remove(i);
        } else {
            i += 1;
        }
    }
    // Seasonal factor function: f(biome) = 1 + A * sin(2π t/T + phase)
    let t = crate_current_step();
    let season_factor = |biome: usize| -> f32 { seasonal_factor_for_biome(t, biome) };

    // Random respawn attempt (biome- and season-scaled): let favorable seasons refill faster
    if food.len() < max_food() {
        // Sample a few candidates and pick the one with the highest seasonal weight to bias spawns
        let mut best_p = None;
        let mut best_w = -f32::INFINITY;
        let samples = FOOD_RESPAWN_CANDIDATE_SAMPLES.max(1); // ensure >=1
        for _ in 0..samples {
            let p = rand_pos(rng);
            let biome = biome_index_for_x(p.x);
            let s = seasonal_factor_for_biome(t, biome);
            let w = BIOME_RESPAWN_MULT[biome] as f32 * s.powf(FOOD_RESPAWN_SEASON_EXP);
            if w > best_w { best_w = w; best_p = Some((p, biome, s)); }
        }
        if let Some((p, biome, s)) = best_p {
            let prob = food_respawn_prob() * BIOME_RESPAWN_MULT[biome] * s.powf(FOOD_RESPAWN_SEASON_EXP);
            if rng.random_range(0.0..1.0) < prob {
                if can_place_food(food, p) { food.push(p); food_life.push(sample_lifetime_for_pos(rng, p)); }
            }
        }
    }
    // Spread from existing foods (biome-scaled)
    let base_len = food.len();
    for idx in 0..base_len {
        if food.len() >= max_food() { break; }
        let parent = food[idx];
        let biome = biome_index_for_x(parent.x);
        // Seasonal push on spread as well
        let prob = FOOD_SPREAD_CHANCE * BIOME_SPREAD_MULT[biome] * season_factor(biome).powf(FOOD_SPREAD_SEASON_EXP);
        if rng.random_range(0.0..1.0) < prob {
            // Spawn near and assign lifetime
            if food.len() < max_food() {
                let before = food.len();
                try_spawn_food_near(food, rng, parent);
                if food.len() > before {
                    let p = food[food.len()-1];
                    food_life.push(sample_lifetime_for_pos(rng, p));
                }
            }
        }
    }
}

fn plant_body_at(pos: Vec2) -> Plant {
    Plant { body: Body { pos, vel: Vec2::new(0.0, 0.0), radius: FOOD_RADIUS } }
}

pub fn eat_if_near(food: &mut Vec<Vec2>, food_life: &mut Vec<usize>, agent_body: &Body) -> bool {
    if food.is_empty() { return false; }
    if let Some((idx, _)) = food.iter().enumerate()
        .map(|(i, f)| (i, plant_body_at(*f)))
        .filter(|(_, plant)| Body::collides(agent_body, &plant.body))
        .map(|(i, plant)| {
            // Return distance for min_by
            let d = (plant.body.pos - agent_body.pos).length();
            (i, d)
        })
        .min_by(|a, b| a.1.total_cmp(&b.1)) {
        food.swap_remove(idx);
        food_life.swap_remove(idx);
        true
    } else { false }
}

// Continuous collision: did the path from p0 to p1 pass within eat radius of any food?
#[allow(dead_code)]
pub fn eat_along_path(food: &mut Vec<Vec2>, food_life: &mut Vec<usize>, p0: Vec2, p1: Vec2, agent_radius: f32) -> bool {
    if food.is_empty() { return false; }
    let (vx, vy) = (p1.x - p0.x, p1.y - p0.y);
    let v_len2 = vx*vx + vy*vy;
    let eat_r = FOOD_RADIUS + agent_radius;
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
    if let Some(i) = best_i { food.swap_remove(i); food_life.swap_remove(i); true } else { false }
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

pub fn wrap_to_world(pos: Vec2) -> Vec2 {
    Vec2::new(
        pos.x.rem_euclid(world_w()),
        pos.y.rem_euclid(world_h()),
    )
}



