use super::params::{VISION_RAYS, VISION_ANGLE_DEG, VISION_RANGE, FOOD_RADIUS, FOOD_VECTOR_MAX_RANGE, DANGER_VECTOR_MAX_RANGE, DENSITY_SECTORS, DENSITY_RADIUS, INPUTS, WORLD_W, WORLD_H, PREDATION_ENABLED, SCAVENGE_ENABLED};
use super::Vec2;

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

pub fn nearest_meat_along_ray(p: Vec2, dir: Vec2, snapshot: &[(Vec2, bool, bool)], self_idx: usize) -> Option<f32> {
    let mut best: Option<f32> = None;
    for (j, (pos, alive, consumed)) in snapshot.iter().enumerate() {
        if j == self_idx { continue; }
        let edible = (*alive && PREDATION_ENABLED) || (!*alive && !*consumed && SCAVENGE_ENABLED);
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

pub fn nearest_food_vector_local(pos: Vec2, theta: f32, food: &[Vec2]) -> (f32, f32) {
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
    let c = theta.cos(); let s = theta.sin();
    let fwd_x = c; let fwd_y = s;
    let right_x = -s; let right_y = c;
    let dot_fwd = (best_v.x * fwd_x + best_v.y * fwd_y) / (d.max(1e-6));
    let dot_right = (best_v.x * right_x + best_v.y * right_y) / (d.max(1e-6));
    (dot_right * att, dot_fwd * att)
}

pub fn nearest_agent_vector_local(pos: Vec2, theta: f32, snapshot: &[(Vec2, bool, bool)], self_idx: usize) -> (f32, f32) {
    let mut best_d2 = f32::INFINITY;
    let mut best_v = Vec2 { x: 0.0, y: 0.0 };
    for (j, (p, alive, consumed)) in snapshot.iter().enumerate() {
        if j == self_idx { continue; }
        if !*alive || *consumed { continue; }
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

pub fn meat_vector_from_rays(pos: Vec2, theta: f32, snapshot: &[(Vec2, bool, bool)], self_idx: usize) -> (f32, f32) {
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

pub fn density_sectors(pos: Vec2, theta: f32, snapshot: &[(Vec2, bool, bool)], self_idx: usize) -> [f32; DENSITY_SECTORS] {
    let mut bins = [0.0f32; DENSITY_SECTORS];
    let two_pi = std::f32::consts::PI * 2.0;
    let sector_size = two_pi / (DENSITY_SECTORS as f32);
    for (j, (p, alive, consumed)) in snapshot.iter().enumerate() {
        if j == self_idx { continue; }
        if !*alive || *consumed { continue; }
        let dx = p.x - pos.x; let dy = p.y - pos.y;
        let d2 = dx*dx + dy*dy; let r2 = DENSITY_RADIUS * DENSITY_RADIUS;
        if d2 > r2 { continue; }
        let d = d2.sqrt().max(1e-6);
        let c = theta.cos(); let s = theta.sin();
        let right = dx * (-s) + dy * c;
        let fwd = dx * c + dy * s;
        let ang = fwd.atan2(right);
        let mut ang2 = ang + std::f32::consts::PI;
        if ang2 < 0.0 { ang2 += two_pi; }
        let idx = (ang2 / sector_size).floor() as usize % DENSITY_SECTORS;
        let w = (1.0 - (d / DENSITY_RADIUS)).clamp(0.0, 1.0);
        bins[idx] = (bins[idx] + w).clamp(0.0, 1.0);
    }
    bins
}

pub fn nearest_food_distance(pos: Vec2, food: &[Vec2]) -> Option<f32> {
    let mut best_d2 = f32::INFINITY;
    for f in food {
        let dx = f.x - pos.x; let dy = f.y - pos.y;
        let d2 = dx*dx + dy*dy;
        if d2 < best_d2 { best_d2 = d2; }
    }
    if best_d2.is_finite() && best_d2 < f32::INFINITY { Some(best_d2.sqrt()) } else { None }
}

pub fn build_inputs(pos: Vec2, theta: f32, food: &[Vec2], energy: f32, last_food_mem: Vec2, last_danger_mem: Vec2, density: &[f32], snapshot: &[(Vec2, bool, bool)], self_idx: usize) -> [f32; INPUTS] {
    let mut inputs = [0.0f32; INPUTS];
    let dir = dir_from_theta(theta);
    let rays = ray_directions(dir);
    let mut k = 0;
    for r in rays {
        let len = (r.x * r.x + r.y * r.y).sqrt().max(1e-6);
        let rdir = Vec2 { x: r.x / len, y: r.y / len };
        let food_t = nearest_food_along_ray(pos, rdir, food);
        let food_sig = food_t.map(|t| 1.0 - (t / VISION_RANGE)).unwrap_or(0.0);
        let wall_t = ray_wall_distance(pos, rdir);
        let wall_sig = if wall_t.is_finite() { (1.0 - (wall_t / VISION_RANGE)).clamp(0.0, 1.0) } else { 0.0 };
        let meat_t = nearest_meat_along_ray(pos, rdir, snapshot, self_idx);
        let meat_sig = meat_t.map(|t| 1.0 - (t / VISION_RANGE)).unwrap_or(0.0);
        inputs[k] = food_sig; k += 1; inputs[k] = wall_sig; k += 1; inputs[k] = meat_sig; k += 1;
    }
    inputs[k] = energy.clamp(0.0, 1.0); k += 1;
    inputs[k] = last_food_mem.x; k += 1; inputs[k] = last_food_mem.y; k += 1;
    inputs[k] = last_danger_mem.x; k += 1; inputs[k] = last_danger_mem.y; k += 1;
    for s in 0..DENSITY_SECTORS { inputs[k] = *density.get(s).unwrap_or(&0.0); k += 1; }
    inputs
}
