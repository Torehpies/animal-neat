use super::params::{
    WORLD_W, WORLD_H,
    FOOD_COUNT, FOOD_MIN_SEP, MAX_FOOD,
    FOOD_RESPAWN_PROB, FOOD_SPREAD_CHANCE, FOOD_SPREAD_RADIUS,
    FOOD_RADIUS,
    BIOME_X_SPLITS, BIOME_RESPAWN_MULT, BIOME_SPREAD_MULT,
    SEASONAL_ENABLED, SEASONAL_PERIOD_STEPS, SEASONAL_AMPLITUDE, BIOME_SEASON_PHASE,
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
    if food.len() < max_food() {
        let p = rand_pos(rng);
        let biome = biome_index_for_x(p.x);
        let prob = food_respawn_prob() * BIOME_RESPAWN_MULT[biome] * season_factor(biome);
        if rng.random_range(0.0..1.0) < prob { if can_place_food(food, p) { food.push(p); } }
    }
    // Spread from existing foods (biome-scaled)
    let base_len = food.len();
    for i in 0..base_len {
        if food.len() >= max_food() { break; }
        let parent = food[i];
        let biome = biome_index_for_x(parent.x);
        let prob = FOOD_SPREAD_CHANCE * BIOME_SPREAD_MULT[biome] * season_factor(biome);
        if rng.random_range(0.0..1.0) < prob {
            try_spawn_food_near(food, rng, parent);
        }
    }
}

fn plant_body_at(pos: Vec2) -> Plant {
    Plant { body: Body { pos, vel: Vec2::new(0.0, 0.0), radius: FOOD_RADIUS } }
}

pub fn eat_if_near(food: &mut Vec<Vec2>, agent_body: &Body) -> bool {
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
        true
    } else { false }
}

// Continuous collision: did the path from p0 to p1 pass within eat radius of any food?
pub fn eat_along_path(food: &mut Vec<Vec2>, p0: Vec2, p1: Vec2, agent_radius: f32) -> bool {
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

pub fn wrap_to_world(pos: Vec2) -> Vec2 {
    Vec2::new(
        pos.x.rem_euclid(world_w()),
        pos.y.rem_euclid(world_h()),
    )
}



