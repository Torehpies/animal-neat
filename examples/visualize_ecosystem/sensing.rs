//! Sensing utilities for the visualize_ecosystem example.
//!
//! Responsibilities (revised vision model):
//! - Build the neural input vector from world/agent state
//! - Provide nearest-object vision distances per (sector × category)
//! - Update hearing sectors from broadcast calls
//!
//! Removed legacy components:
//! - Pooled sector proximities (replaced by nearest-distance encoding)
//! - Density sectors (congestion awareness) - simplified out for now
//!
//! Vision encoding:
//!   Sectors: Left, Forward, Right (L,F,R)
//!   Categories per sector: Plant, Carcass, SameAlive, OtherAlive, Wall
//!   Value: normalized distance d / VISION_RANGE in [0,1]; 1.0 = none seen.
//!          (Previously we used proximity strengths; caller can recover a proximity-like
//!           signal via 1 - distance if desired for UI.)

use std::ops::Range;

/// Ranges defining the indices of each modality within the neural network input vector.
/// Keep this single source of truth in sync with params::INPUTS and modality counts.
pub struct InputRanges {
    pub vision: Range<usize>,    // VISION_RAYS × 5 categories
    pub energy: usize,           // single scalar
    pub memory: Range<usize>,    // 6 (food_x, food_y, same_x, same_y, other_x, other_y)
    pub hearing: Range<usize>,   // HEARING_SECTORS
    pub position: Range<usize>,  // 2 (x/WORLD_W, y/WORLD_H)
}

/// Compute and return the current input layout ranges (derived from params).
pub fn input_ranges() -> InputRanges {
    use super::params::*;
    let vision_len = VISION_RAYS * 5;
    let vision = 0..vision_len; // per-ray × category
    let energy = vision.end;
    let memory = (energy + 1)..(energy + 1 + 6);
    let hearing = memory.end..(memory.end + HEARING_SECTORS);
    let position = hearing.end..(hearing.end + 2);
    InputRanges { vision, energy, memory, hearing, position }
}

use super::params::{VISION_RAYS, VISION_ANGLE_DEG, VISION_RANGE, FOOD_RADIUS, AGENT_COLLISION_RADIUS, DANGER_VECTOR_MAX_RANGE, INPUTS, WORLD_W, WORLD_H, HEARING_SECTORS, SOUND_RANGE, SOUND_ATTENUATION_EXP, HEARING_EMA_ALPHA};
use crate::sim::{Agent};
use macroquad::prelude::Vec2;

fn dir_from_theta(theta: f32) -> Vec2 { Vec2 { x: theta.cos(), y: theta.sin() } }

pub fn ray_directions(dir: Vec2) -> Vec<Vec2> {
    let center_ang = dir.y.atan2(dir.x);
    let half = VISION_ANGLE_DEG.to_radians() * 0.5; let start = center_ang - half;
    let step = if VISION_RAYS > 1 { (2.0 * half) / (VISION_RAYS as f32 - 1.0) } else { 0.0 };
    (0..VISION_RAYS).map(|i| { let ang = start + step * (i as f32); Vec2 { x: ang.cos(), y: ang.sin() } }).collect()
}

pub fn ray_wall_distance(p: Vec2, dir: Vec2) -> f32 {
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

pub fn nearest_food_along_ray(p: Vec2, dir: Vec2, food: &[Vec2]) -> Option<f32> {
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

#[allow(dead_code)]
pub fn nearest_agent_vector_local(pos: Vec2, theta: f32, snapshot: &[(Vec2, bool, bool, usize, bool)], self_idx: usize) -> (f32, f32) {
    let mut best_d2 = f32::INFINITY;
    let mut best_v = Vec2 { x: 0.0, y: 0.0 };
    for (j, (p, alive, consumed, _species, is_corpse)) in snapshot.iter().enumerate() {
        if j == self_idx { continue; }
        if !*alive || *consumed || *is_corpse { continue; }
        let dx = p.x - pos.x; let dy = p.y - pos.y;
        let d2 = dx*dx + dy*dy;
        if d2 < best_d2 { best_d2 = d2; best_v = Vec2 { x: dx, y: dy }; }
    }
    if !best_d2.is_finite() || best_d2.is_infinite() { return (0.0, 0.0); }
    let d = best_d2.sqrt();
    let att = (1.0 - (d / DANGER_VECTOR_MAX_RANGE)).clamp(0.0, 1.0);
    if att <= 0.0 { return (0.0, 0.0); }
    let c = theta.cos(); let s = theta.sin();
    let fwd_x = c; let fwd_y = s;
    let right_x = -s; let right_y = c;
    let dot_fwd = (best_v.x * fwd_x + best_v.y * fwd_y) / (d.max(1e-6));
    let dot_right = (best_v.x * right_x + best_v.y * right_y) / (d.max(1e-6));
    (dot_right * att, dot_fwd * att)
}

pub fn nearest_same_other_vectors_local(pos: Vec2, theta: f32, snapshot: &[(Vec2, bool, bool, usize, bool)], self_idx: usize, my_species: usize) -> ((f32,f32),(f32,f32)) {
    let mut best_same = (f32::INFINITY, Vec2 { x: 0.0, y: 0.0 });
    let mut best_other = (f32::INFINITY, Vec2 { x: 0.0, y: 0.0 });
    for (j, (p, alive, consumed, species, is_corpse)) in snapshot.iter().enumerate() {
        if j == self_idx { continue; }
        if !*alive || *consumed || *is_corpse { continue; }
        let dx = p.x - pos.x; let dy = p.y - pos.y;
        let d2 = dx*dx + dy*dy;
        if *species == my_species {
            if d2 < best_same.0 { best_same = (d2, Vec2 { x: dx, y: dy }); }
        } else {
            if d2 < best_other.0 { best_other = (d2, Vec2 { x: dx, y: dy }); }
        }
    }
    let rot = |v: Vec2, d2: f32| -> (f32,f32) {
        if !d2.is_finite() || d2.is_infinite() { return (0.0, 0.0); }
        let d = d2.sqrt();
        let att = (1.0 - (d / DANGER_VECTOR_MAX_RANGE)).clamp(0.0, 1.0);
        if att <= 0.0 { return (0.0, 0.0); }
        let c = theta.cos(); let s = theta.sin();
        let fwd_x = c; let fwd_y = s; let right_x = -s; let right_y = c;
        let dot_fwd = (v.x * fwd_x + v.y * fwd_y) / d.max(1e-6);
        let dot_right = (v.x * right_x + v.y * right_y) / d.max(1e-6);
        (dot_right * att, dot_fwd * att)
    };
    let same = if best_same.0.is_finite() { rot(best_same.1, best_same.0) } else { (0.0, 0.0) };
    let other = if best_other.0.is_finite() { rot(best_other.1, best_other.0) } else { (0.0, 0.0) };
    (same, other)
}

// Optional: derive a local vector from rays for UI/memory (weighted by signal strength along rays)
pub fn aggregate_vector_from_rays(theta: f32, hits: &[(Vec2, f32)]) -> (f32, f32) {
    // hits: list of (dir_world, strength in 0..1). Convert to local and average.
    if hits.is_empty() { return (0.0, 0.0); }
    let c = theta.cos(); let s = theta.sin();
    let right_x = -s; let right_y = c;
    let fwd_x = c; let fwd_y = s;
    let mut sx = 0.0f32; let mut sy = 0.0f32; let mut sw = 0.0f32;
    for (dir_w, w) in hits {
        let lx = dir_w.x * right_x + dir_w.y * right_y;
        let ly = dir_w.x * fwd_x + dir_w.y * fwd_y;
        sx += lx * *w; sy += ly * *w; sw += *w;
    }
    if sw <= 1e-6 { return (0.0, 0.0); }
    let mut lx = sx / sw; let mut ly = sy / sw;
    let len = (lx*lx + ly*ly).sqrt().max(1e-6);
    lx /= len; ly /= len; // unit in local frame
    (lx, ly)
}

pub fn food_vector_from_rays(pos: Vec2, theta: f32, food: &[Vec2]) -> (f32, f32) {
    let dir = dir_from_theta(theta);
    let rays = ray_directions(dir);
    let mut hits: Vec<(Vec2, f32)> = Vec::with_capacity(rays.len());
    for r in rays {
        let len = (r.x * r.x + r.y * r.y).sqrt().max(1e-6);
        let rdir = Vec2 { x: r.x / len, y: r.y / len };
        if let Some(t) = nearest_food_along_ray(pos, rdir, food) {
            let w = (1.0 - (t / VISION_RANGE)).clamp(0.0, 1.0);
            if w > 0.0 { hits.push((rdir, w)); }
        }
    }
    aggregate_vector_from_rays(theta, &hits)
}

pub fn build_inputs_inplace(
    out: &mut [f32],
    pos: Vec2,
    theta: f32,
    food: &[Vec2],
    energy: f32,
    last_food_mem: Vec2,
    last_same_mem: Vec2,
    last_other_mem: Vec2,
    snapshot: &[(Vec2, bool, bool, usize, bool)],
    self_idx: usize,
    my_species: usize,
    heard: [f32;3],
    food_indices: Option<&[usize]>,
    agent_indices: Option<&[usize]>,
) {
    debug_assert_eq!(out.len(), INPUTS);
    // Compute ranges dynamically to avoid hard-coded indices
    let ranges = input_ranges();
    // Initialize all inputs to default zeros, then fill sections
    for v in out.iter_mut() { *v = 0.0; }
    // Vision per-ray × category, default = 1.0 (nothing seen)
    for i in ranges.vision.clone() { out[i] = 1.0; }
    let dir_center = dir_from_theta(theta);
    let rays = ray_directions(dir_center);
    // Optional candidate filters
    let food_iter: Box<dyn Iterator<Item=&Vec2>> = if let Some(idxs) = food_indices {
        Box::new(idxs.iter().filter_map(move |i| food.get(*i)))
    } else { Box::new(food.iter()) };
    let food_list: Vec<Vec2> = food_iter.cloned().collect();
    let agent_iter: Box<dyn Iterator<Item=(usize,&(Vec2,bool,bool,usize,bool))>> = if let Some(idxs) = agent_indices {
        Box::new(idxs.iter().filter_map(move |j| snapshot.get(*j).map(|v| (*j,v))))
    } else { Box::new(snapshot.iter().enumerate()) };
    let agent_list: Vec<(usize, (Vec2,bool,bool,usize,bool))> = agent_iter.map(|(j,t)| (j, (*t).clone())).collect();

    for (ri, r) in rays.iter().enumerate() {
        // Normalize direction
        let len = (r.x * r.x + r.y * r.y).sqrt().max(1e-6);
        let rdir = Vec2 { x: r.x / len, y: r.y / len };
        // Accumulators for nearest t per category
        let mut best = [None; 5]; // 0=plant,1=carcass,2=same,3=other,4=wall
        // Plants along this ray
        for fpos in &food_list {
            let op = Vec2 { x: fpos.x - pos.x, y: fpos.y - pos.y };
            let t = op.x * rdir.x + op.y * rdir.y;
            if t <= 0.0 || t > VISION_RANGE { continue; }
            let closest = Vec2 { x: pos.x + rdir.x * t, y: pos.y + rdir.y * t };
            let dx = fpos.x - closest.x; let dy = fpos.y - closest.y; let dist = (dx*dx + dy*dy).sqrt();
            if dist <= FOOD_RADIUS { match best[0] { Some(b) if t >= b => {}, _ => best[0] = Some(t) } }
        }
        // Agents/carcasses along this ray (pure perception, not gated by predation/scavenge flags)
        for (j, (apos, alive, consumed, species_id, is_corpse)) in &agent_list {
            if *j == self_idx || *consumed { continue; }
            let op = Vec2 { x: apos.x - pos.x, y: apos.y - pos.y };
            let t = op.x * rdir.x + op.y * rdir.y;
            if t <= 0.0 || t > VISION_RANGE { continue; }
            let closest = Vec2 { x: pos.x + rdir.x * t, y: pos.y + rdir.y * t };
            let dx = apos.x - closest.x; let dy = apos.y - closest.y; let dist = (dx*dx + dy*dy).sqrt();
            if dist <= AGENT_COLLISION_RADIUS {
                if *alive {
                    if *species_id == my_species { match best[2] { Some(b) if t >= b => {}, _ => best[2] = Some(t) } }
                    else { match best[3] { Some(b) if t >= b => {}, _ => best[3] = Some(t) } }
                } else if *is_corpse { match best[1] { Some(b) if t >= b => {}, _ => best[1] = Some(t) } }
            }
        }
        // Wall along this ray (optional)
        if super::params::ENABLE_VISION_WALLS {
            let tw = ray_wall_distance(pos, rdir);
            if tw.is_finite() && tw > 0.0 && tw <= VISION_RANGE { best[4] = Some(best[4].map_or(tw, |b| b.min(tw))); }
        }
        // Write normalized distances into output slice
        let base = ranges.vision.start + ri * 5;
        for cat in 0..5 {
            if let Some(t) = best[cat] { out[base + cat] = (t / VISION_RANGE).clamp(0.0, 1.0); }
        }
    }
    // Energy scalar
    out[ranges.energy] = energy.clamp(0.0, 1.0);
    // Memory (6 floats)
    out[ranges.memory.start + 0] = last_food_mem.x; out[ranges.memory.start + 1] = last_food_mem.y;
    out[ranges.memory.start + 2] = last_same_mem.x; out[ranges.memory.start + 3] = last_same_mem.y;
    out[ranges.memory.start + 4] = last_other_mem.x; out[ranges.memory.start + 5] = last_other_mem.y;
    // Hearing
    for si in 0..HEARING_SECTORS { out[ranges.hearing.start + si] = heard[si].clamp(0.0,1.0); }
    // Position
    out[ranges.position.start] = (pos.x / WORLD_W).clamp(0.0,1.0);
    out[ranges.position.start + 1] = (pos.y / WORLD_H).clamp(0.0,1.0);
}

pub fn build_inputs(pos: Vec2, theta: f32, food: &[Vec2], energy: f32, last_food_mem: Vec2, last_same_mem: Vec2, last_other_mem: Vec2, snapshot: &[(Vec2, bool, bool, usize, bool)], self_idx: usize, my_species: usize, heard: [f32;3]) -> [f32; INPUTS] {
    let mut arr = [0.0f32; INPUTS];
    build_inputs_inplace(&mut arr, pos, theta, food, energy, last_food_mem, last_same_mem, last_other_mem, snapshot, self_idx, my_species, heard, None, None);
    arr
}

/// Update hearing sectors for all agents based on others' call_intensity
pub fn update_hearing(agents: &mut [Agent]) {
    if agents.is_empty() { return; }
    // Precompute facing vectors for sector classification
    for i in 0..agents.len() {
        let (c, s) = (agents[i].theta.cos(), agents[i].theta.sin());
        let fwd = Vec2 { x: c, y: s }; let right = Vec2 { x: -s, y: c };
        let half = VISION_ANGLE_DEG.to_radians() * 0.5;
        let forward_band = half / 6.0; // reuse same logic as vision sectors
        let mut accum = [0.0f32;3];
        for (j, other) in agents.iter().enumerate() { if i == j { continue; }
            let dx = other.body.pos.x - agents[i].body.pos.x; let dy = other.body.pos.y - agents[i].body.pos.y;
            let d2 = dx*dx + dy*dy; let r2 = SOUND_RANGE * SOUND_RANGE; if d2 > r2 || other.call_intensity <= 1e-6 { continue; }
            let d = d2.sqrt().max(1e-6);
            let fwd_comp = dx * fwd.x + dy * fwd.y; if fwd_comp <= 0.0 { continue; } // only front hemisphere for directional hearing (simplification)
            let right_comp = dx * right.x + dy * right.y; let ang = right_comp.atan2(fwd_comp);
            if ang < -half || ang > half { continue; }
            let si = if ang < -forward_band { 0 } else if ang <= forward_band { 1 } else { 2 };
            let base = (1.0f32 - (d / SOUND_RANGE)).clamp(0.0f32, 1.0f32).powf(SOUND_ATTENUATION_EXP);
            let weight = base * other.call_intensity; // linear mix; could square intensity if desired
            accum[si] += weight;
        }
        // normalize/clamp and apply smoothing EMA
        for si in 0..3 { let v = accum[si].min(1.0); agents[i].heard_sectors[si] = agents[i].heard_sectors[si] + HEARING_EMA_ALPHA * (v - agents[i].heard_sectors[si]); }
    }
}
