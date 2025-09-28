//! Sensing utilities for the visualize_ecosystem example.
//!
//! Responsibilities:
//! - Build the neural input vector from world/agent state
//! - Compute pooled sector proximities (vision)
//! - Compute density sectors around an agent
//! - Update hearing sectors from broadcast calls
//!
//! This module now exposes `input_ranges()` to centralize the layout of the
//! input vector, avoiding magic indices sprinkled across files.

use std::ops::Range;

/// Ranges defining the indices of each modality within the neural network input vector.
/// Keep this single source of truth in sync with params::INPUTS and modality counts.
pub struct InputRanges {
    pub vision: Range<usize>,    // 3 sectors × 4 categories = 12
    pub energy: usize,           // single scalar
    pub memory: Range<usize>,    // 4 (food_x, food_y, danger_x, danger_y)
    pub density: Range<usize>,   // DENSITY_SECTORS
    pub hearing: Range<usize>,   // HEARING_SECTORS
    pub position: Range<usize>,  // 2 (x/WORLD_W, y/WORLD_H)
}

/// Compute and return the current input layout ranges (derived from params).
pub fn input_ranges() -> InputRanges {
    use super::params::*;
    let vision = 0..12;
    let energy = 12;
    let memory = 13..17;
    let density = 17..(17 + DENSITY_SECTORS);
    let hearing = density.end..(density.end + HEARING_SECTORS);
    let position = hearing.end..(hearing.end + 2);
    InputRanges { vision, energy, memory, density, hearing, position }
}
use super::params::{VISION_RAYS, VISION_ANGLE_DEG, VISION_RANGE, FOOD_RADIUS, DANGER_VECTOR_MAX_RANGE, DENSITY_SECTORS, DENSITY_RADIUS, INPUTS, WORLD_W, WORLD_H, PREDATION_ENABLED, SCAVENGE_ENABLED, HEARING_SECTORS, SOUND_RANGE, SOUND_ATTENUATION_EXP, HEARING_EMA_ALPHA};
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

pub fn nearest_meat_along_ray(p: Vec2, dir: Vec2, snapshot: &[(Vec2, bool, bool, usize, bool)], self_idx: usize) -> Option<f32> {
    let mut best: Option<f32> = None;
    for (j, (pos, alive, consumed, _species, is_corpse)) in snapshot.iter().enumerate() {
        if j == self_idx { continue; }
        let edible = (*alive && PREDATION_ENABLED) || (*is_corpse && !*consumed && SCAVENGE_ENABLED);
        if !edible { continue; }
        let op = Vec2 { x: pos.x - p.x, y: pos.y - p.y };
        let t = op.x * dir.x + op.y * dir.y;
        if t <= 0.0 || t > VISION_RANGE { continue; }
        // distance from ray to point must be within agent-meat interaction scale ~ FOOD_RADIUS for consistency
        let closest = Vec2 { x: p.x + dir.x * t, y: p.y + dir.y * t };
        let dx = pos.x - closest.x; let dy = pos.y - closest.y; let dist = (dx*dx + dy*dy).sqrt();
        if dist <= FOOD_RADIUS { match best { Some(b) if t >= b => {}, _ => best = Some(t) } }
    }
    best
}

// Removed unused nearest_food_vector_local (legacy shaping vector) to reduce warnings.

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

pub fn meat_vector_from_rays(pos: Vec2, theta: f32, snapshot: &[(Vec2, bool, bool, usize, bool)], self_idx: usize) -> (f32, f32) {
    let dir = dir_from_theta(theta);
    let rays = ray_directions(dir);
    let mut hits: Vec<(Vec2, f32)> = Vec::with_capacity(rays.len());
    for r in rays {
        let len = (r.x * r.x + r.y * r.y).sqrt().max(1e-6);
        let rdir = Vec2 { x: r.x / len, y: r.y / len };
        if let Some(t) = nearest_meat_along_ray(pos, rdir, snapshot, self_idx) {
            let w = (1.0 - (t / VISION_RANGE)).clamp(0.0, 1.0);
            if w > 0.0 { hits.push((rdir, w)); }
        }
    }
    aggregate_vector_from_rays(theta, &hits)
}

pub fn density_sectors(pos: Vec2, theta: f32, snapshot: &[(Vec2, bool, bool, usize, bool)], self_idx: usize) -> [f32; DENSITY_SECTORS] {
    // 6-bin layout:
    // 0: forward (within 45° of forward)
    // 1: side-left (45°..135° left)
    // 2: side-right (45°..135° right)
    // 3: back-left (135°..180° / -180°..-135° left region folded)
    // 4: back-right (135°..180° / -180°..-135° right region folded)
    // 5: back-center (within 45° of directly behind)
    let mut bins = [0.0f32; DENSITY_SECTORS];
    for (j, (p, alive, consumed, _species, _is_corpse)) in snapshot.iter().enumerate() {
        if j == self_idx { continue; }
        if !*alive || *consumed { continue; }
        let dx = p.x - pos.x; let dy = p.y - pos.y;
        let d2 = dx*dx + dy*dy; let r2 = DENSITY_RADIUS * DENSITY_RADIUS; if d2 > r2 { continue; }
        let d = d2.sqrt().max(1e-6);
        let c = theta.cos(); let s = theta.sin();
        let fwd = dx * c + dy * s; // forward component
        let right = dx * (-s) + dy * c; // right component
        // local angle where 0 = forward, positive = left (we construct using atan2(left,right) style)
        let ang = right.atan2(fwd); // range -PI..PI, 0 forward, +PI/2 left, -PI/2 right
        let w = (1.0 - (d / DENSITY_RADIUS)).clamp(0.0, 1.0);
        use std::f32::consts::{FRAC_PI_4, FRAC_PI_2, PI};
        let a = ang;
        let add = |bin: &mut f32, val: f32| { *bin = (*bin + val).clamp(0.0, 1.0); };
        if a.abs() <= FRAC_PI_4 { add(&mut bins[0], w); } // forward
        else if a > FRAC_PI_4 && a <= FRAC_PI_2 + FRAC_PI_4 { add(&mut bins[1], w); } // side-left
        else if a < -FRAC_PI_4 && a >= -FRAC_PI_2 - FRAC_PI_4 { add(&mut bins[2], w); } // side-right
        else {
            // back hemisphere: distinguish center vs sides
            let back_ang = if a >= 0.0 { PI - a } else { PI + a }; // 0 at directly back
            if back_ang <= FRAC_PI_4 { add(&mut bins[5], w); } // back-center
            else if a > 0.0 { add(&mut bins[3], w); } else { add(&mut bins[4], w); }
        }
    }
    bins
}

// Removed unused nearest_food_distance (legacy diagnostic) to reduce warnings.

pub fn build_inputs(pos: Vec2, theta: f32, food: &[Vec2], energy: f32, last_food_mem: Vec2, last_danger_mem: Vec2, density: &[f32], snapshot: &[(Vec2, bool, bool, usize, bool)], self_idx: usize, my_species: usize, heard: [f32;3]) -> [f32; INPUTS] {
    let pools = compute_sector_pools(pos, theta, food, snapshot, self_idx, my_species);
    let mut inputs = [0.0f32; INPUTS];
    let mut k = 0;
    for si in 0..3 {
        inputs[k] = pools.plant_carc[si]; k += 1;
        inputs[k] = pools.same_alive[si]; k += 1;
        inputs[k] = pools.other_alive[si]; k += 1;
        inputs[k] = pools.wall[si]; k += 1;
    }
    inputs[k] = energy.clamp(0.0, 1.0); k += 1;
    inputs[k] = last_food_mem.x; k += 1; inputs[k] = last_food_mem.y; k += 1;
    inputs[k] = last_danger_mem.x; k += 1; inputs[k] = last_danger_mem.y; k += 1;
    for s in 0..DENSITY_SECTORS { inputs[k] = *density.get(s).unwrap_or(&0.0); k += 1; }
    // hearing sectors (already smoothed)
    for si in 0..HEARING_SECTORS { inputs[k] = heard[si].clamp(0.0, 1.0); k += 1; }
    // normalized absolute position (helps with navigation / region strategies)
    inputs[k] = (pos.x / WORLD_W).clamp(0.0, 1.0); k += 1;
    inputs[k] = (pos.y / WORLD_H).clamp(0.0, 1.0);
    inputs
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

// Public struct for pooled sector proximities so UI can reuse without duplicating logic
#[derive(Clone, Copy, Debug)]
pub struct SectorPools {
    pub plant_carc: [f32;3],
    pub same_alive: [f32;3],
    pub other_alive: [f32;3],
    pub wall: [f32;3],
}

impl SectorPools {
    pub fn empty() -> Self { Self { plant_carc: [0.0;3], same_alive: [0.0;3], other_alive: [0.0;3], wall: [0.0;3] } }
}

/// Compute directional pooled proximities (Left, Forward, Right) × (Plant/Carcass, Same, Other, Wall)
pub fn compute_sector_pools(pos: Vec2, theta: f32, food: &[Vec2], snapshot: &[(Vec2, bool, bool, usize, bool)], self_idx: usize, my_species: usize) -> SectorPools {
    let half_cone = VISION_ANGLE_DEG.to_radians() * 0.5;
    let forward_band = half_cone / 6.0;
    let c = theta.cos(); let s = theta.sin();
    let fwd = Vec2 { x: c, y: s }; let right_vec = Vec2 { x: -s, y: c };
    let sector_index = |ang: f32| -> Option<usize> {
        if ang < -half_cone || ang > half_cone { return None; }
        if ang < -forward_band { Some(0) } else if ang <= forward_band { Some(1) } else { Some(2) }
    };
    let mut plant_carc = [0.0f32;3];
    let mut same_alive = [0.0f32;3];
    let mut other_alive = [0.0f32;3];
    let mut wall_prox = [0.0f32;3];

    // Plants
    for fpos in food {
        let dx = fpos.x - pos.x; let dy = fpos.y - pos.y;
        let dist2 = dx*dx + dy*dy; if dist2 > VISION_RANGE * VISION_RANGE { continue; }
        let dist = dist2.sqrt().max(1e-6);
        let fwd_comp = dx * fwd.x + dy * fwd.y; if fwd_comp <= 0.0 { continue; }
        let right_comp = dx * right_vec.x + dy * right_vec.y; let ang = right_comp.atan2(fwd_comp);
    if let Some(si) = sector_index(ang) { let base = (1.0 - dist / VISION_RANGE).clamp(0.0,1.0); let w = base * base; if w > plant_carc[si] { plant_carc[si] = w; } }
    }

    // Agents / carcasses
    for (j, (apos, alive, consumed, species_id, is_corpse)) in snapshot.iter().enumerate() {
        if j == self_idx { continue; }
        let dx = apos.x - pos.x; let dy = apos.y - pos.y; let dist2 = dx*dx + dy*dy; if dist2 > VISION_RANGE * VISION_RANGE { continue; }
        let dist = dist2.sqrt().max(1e-6);
        let fwd_comp = dx * fwd.x + dy * fwd.y; if fwd_comp <= 0.0 { continue; }
        let right_comp = dx * right_vec.x + dy * right_vec.y; let ang = right_comp.atan2(fwd_comp);
        if let Some(si) = sector_index(ang) {
            let base = (1.0 - dist / VISION_RANGE).clamp(0.0,1.0); let w = base * base;
            if *alive {
                if *species_id == my_species { if w > same_alive[si] { same_alive[si] = w; } }
                else { if w > other_alive[si] { other_alive[si] = w; } }
            } else if *is_corpse && !*consumed {
                if w > plant_carc[si] { plant_carc[si] = w; }
            }
        }
    }

    // Walls (sample sector centers)
    let sector_dirs = [ -half_cone * 0.66, 0.0, half_cone * 0.66 ];
    for (si, off) in sector_dirs.iter().enumerate() {
        let ang_world = theta + *off; let dir = Vec2 { x: ang_world.cos(), y: ang_world.sin() };
        let t = ray_wall_distance(pos, dir);
    if t.is_finite() && t>0.0 && t<=VISION_RANGE { let base = (1.0 - t / VISION_RANGE).clamp(0.0,1.0); let w = base * base; wall_prox[si] = w; }
    }

    SectorPools { plant_carc, same_alive, other_alive, wall: wall_prox }
}
