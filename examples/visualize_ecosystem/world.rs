use super::params::{WORLD_W, WORLD_H, FOOD_COUNT, FOOD_MIN_SEP, MAX_FOOD, FOOD_RESPAWN_PROB, FOOD_SPREAD_CHANCE, FOOD_SPREAD_RADIUS, FOOD_RADIUS, AGENT_RADIUS};
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

pub fn try_spawn_food_random<R: Rng>(food: &mut Vec<Vec2>, rng: &mut R) {
    if food.len() >= MAX_FOOD { return; }
    let p = rand_pos(rng);
    if can_place_food(food, p) { food.push(p); }
}

pub fn try_spawn_food_near<R: Rng>(food: &mut Vec<Vec2>, rng: &mut R, center: Vec2) {
    if food.len() >= MAX_FOOD { return; }
    let ang = rng.random_range(0.0..(std::f32::consts::PI * 2.0));
    let r = rng.random_range(0.5..FOOD_SPREAD_RADIUS);
    let p = Vec2 { x: (center.x + ang.cos() * r).clamp(0.0, WORLD_W), y: (center.y + ang.sin() * r).clamp(0.0, WORLD_H) };
    if can_place_food(food, p) { food.push(p); }
}

pub fn food_growth_step<R: Rng>(food: &mut Vec<Vec2>, rng: &mut R) {
    if rng.random_range(0.0..1.0) < FOOD_RESPAWN_PROB { try_spawn_food_random(food, rng); }
    let base_len = food.len();
    for i in 0..base_len {
        if food.len() >= MAX_FOOD { break; }
        if rng.random_range(0.0..1.0) < FOOD_SPREAD_CHANCE {
            let parent = food[i];
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
