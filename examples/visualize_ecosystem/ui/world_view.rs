use macroquad::prelude::*;
use crate::params::*;
use crate::{Episode, Vec2};
use crate::sensing;
use crate::ui_common::{world_to_screen, fit_world_rect, world_scale};
use crate::dir_from_theta;

pub fn draw_world(area: Rect, episode: &Episode, show_cones: bool, _member_species: &[usize], unified_overlay: bool, mouse_world: Option<Vec2>) {
    let fitted = fit_world_rect(area);
    // background: draw biome bands with seasonal tinting
    // Biomes split across X using BIOME_X_SPLITS; use episode.steps as season time
    let splits = BIOME_X_SPLITS;
    let bands = [0.0, splits[0], splits[1], 1.0];
    for b in 0..3 {
        let x0w = bands[b] * WORLD_W;
        let x1w = bands[b + 1] * WORLD_W;
        let (x0, _) = world_to_screen(fitted, Vec2 { x: x0w, y: 0.0 });
        let (x1, _) = world_to_screen(fitted, Vec2 { x: x1w, y: 0.0 });
        let w = (x1 - x0).abs();
        // Base tint per biome + seasonal brightness
        let base = match b {
            0 => Color::new(0.08, 0.20, 0.08, 1.0),
            1 => Color::new(0.09, 0.24, 0.09, 1.0),
            _ => Color::new(0.10, 0.28, 0.10, 1.0),
        };
    let (mut r, mut g, mut bl, a) = (base.r, base.g, base.b, 1.0);
        if SEASONAL_ENABLED && SEASONAL_PERIOD_STEPS > 0 {
            let t = episode.steps as f32 * std::f32::consts::TAU / (SEASONAL_PERIOD_STEPS as f32) + BIOME_SEASON_PHASE[b];
            let s = (1.0 + SEASONAL_AMPLITUDE * t.sin()).max(0.0);
            let brighten = 0.15 * (s - 1.0); // modest seasonal effect
            r = (r + brighten).clamp(0.0, 1.0);
            g = (g + brighten * 1.3).clamp(0.0, 1.0);
            bl = (bl + brighten).clamp(0.0, 1.0);
        }
        draw_rectangle(x0.min(x1), fitted.y, w, fitted.h, Color::new(r, g, bl, a));
    }
    // border
    draw_rectangle_lines(fitted.x, fitted.y, fitted.w, fitted.h, 2.0, BLACK);
    let px_per_world = world_scale(fitted);
    // food
    for p in &episode.food {
        let (px, py) = world_to_screen(fitted, *p);
        let r = (FOOD_RADIUS * px_per_world).max(2.0);
        draw_circle(px, py, r, YELLOW);
    }
    // Precompute snapshot for overlays
    let snapshot: Vec<(Vec2, bool, bool, usize, bool)> = episode.agents.iter().map(|a| {
        let alive = a.energy > 0.0;
        let is_corpse = !alive && !a.consumed && a.corpse_energy > 0.1;
        (a.pos, alive, a.consumed, 0usize, is_corpse)
    }).collect();

    // Determine focused agent (nearest to mouse)
    let focused_idx: Option<usize> = mouse_world.and_then(|mw| {
        let mut best: Option<(usize, f32)> = None;
        for (i, a) in episode.agents.iter().enumerate() {
            if a.energy <= 0.0 { continue; }
            let dx = a.pos.x - mw.x; let dy = a.pos.y - mw.y; let d2 = dx*dx + dy*dy;
            if let Some((_, b)) = best { if d2 < b { best = Some((i, d2)); } } else { best = Some((i, d2)); }
        }
        best.map(|(i, _)| i)
    });

    // agents
    for (idx, a) in episode.agents.iter().enumerate() {
    let (px, py) = world_to_screen(fitted, a.pos);
    let agent_r = (AGENT_RADIUS * px_per_world).max(3.0);
    // species index unused for coloring now that diet-based coloring is applied
        // draw alive vs dead differently
        if a.energy > 0.0 {
            // Color by diet: greener for plant-eaters, redder for meat-eaters.
            // Use episode stats: a.eaten counts all edible events; a.kills counts meat events (live or corpse).
            let meat = a.kills as f32;
            let plants = a.eaten.saturating_sub(a.kills) as f32;
            let total = meat + plants;
            let meat_ratio = if total > 0.0 { meat / total } else { 0.0 };
            let hue = (1.0 / 3.0) * (1.0 - meat_ratio); // 1/3 = green, 0 = red
            let sat = if total > 0.0 { 0.85 } else { 0.25 }; // pale before first meal
            let val = 0.95;
            let (r, g, b) = crate::ui_common::hsv_to_rgb(hue, sat, val);
            let fill = Color::new(r, g, b, 1.0);
            draw_circle(px, py, agent_r, fill);
            // Communication: call emission ring (intensity-based)
            if a.call_intensity > 0.03 {
                let ring_r = agent_r + 6.0 + a.call_intensity * 22.0;
                let alpha = 0.15 + 0.55 * a.call_intensity;
                draw_circle_lines(px, py, ring_r, 2.0, Color::new(0.95, 0.2, 1.0, alpha));
                // Inner pulse (faint fill) for stronger calls
                if a.call_intensity > 0.6 {
                    draw_circle(px, py, agent_r + 4.0, Color::new(0.95, 0.2, 1.0, 0.08 + 0.12 * (a.call_intensity - 0.6)));                    
                }
            }
        } else {
            // If corpse is fully consumed or flagged consumed, skip rendering
            if a.consumed || a.corpse_energy <= 0.1 { continue; }
            let fill = Color::new(0.25, 0.25, 0.25, 0.9);
            draw_circle(px, py, agent_r, fill);
        }
        draw_circle_lines(px, py, agent_r, 2.0, Color::new(0.2, 0.2, 0.2, 0.6));
        // heading line
        if a.energy > 0.0 {
            let dir = dir_from_theta(a.theta);
            let (hx, hy) = world_to_screen(fitted, Vec2 { x: a.pos.x + dir.x * 2.0, y: a.pos.y + dir.y * 2.0 });
            draw_line(px, py, hx, hy, 2.0, BLUE);
        }

        if show_cones && a.energy > 0.0 {
            let dir = dir_from_theta(a.theta);
            for r in sensing::ray_directions(dir) {
                let food_t = sensing::nearest_food_along_ray(a.pos, r, &episode.food);
                let meat_t = sensing::nearest_meat_along_ray(a.pos, r, &snapshot, idx);
                match food_t {
                    Some(t) => {
                        let sense_pt = Vec2 { x: a.pos.x + r.x * t, y: a.pos.y + r.y * t };
                        let (sx, sy) = world_to_screen(fitted, sense_pt);
                        // draw sensed segment in green up to the food point
                        let green = Color::new(0.2, 1.0, 0.2, 0.9);
                        draw_line(px, py, sx, sy, 2.0, green);
                        // mark the sensed point
                        draw_circle(sx, sy, 3.0, green);
                        // faint remainder to max range (lighter green)
                        let end = Vec2 { x: a.pos.x + r.x * VISION_RANGE, y: a.pos.y + r.y * VISION_RANGE };
                        let (x2, y2) = world_to_screen(fitted, end);
                        draw_line(sx, sy, x2, y2, 1.0, Color::new(0.2, 1.0, 0.2, 0.25));
                    }
                    None => {
                        let end = Vec2 { x: a.pos.x + r.x * VISION_RANGE, y: a.pos.y * 1.0 + r.y * VISION_RANGE };
                        let (x2, y2) = world_to_screen(fitted, end);
                        draw_line(px, py, x2, y2, 1.0, Color::new(0.2, 1.0, 0.2, 0.35));
                    }
                }
                // Overlay meat hit (orange) if present on this ray
                if let Some(tm) = meat_t {
                    let mpt = Vec2 { x: a.pos.x + r.x * tm, y: a.pos.y + r.y * tm };
                    let (mx, my) = world_to_screen(fitted, mpt);
                    let orange = Color::new(1.0, 0.6, 0.1, 0.95);
                    draw_line(px, py, mx, my, 2.0, orange);
                    draw_circle(mx, my, 3.0, orange);
                }
            }
        }
        // Visual cue: edible nearby (live prey or unconsumed corpse) within EAT_AGENT_RADIUS
        if a.energy > 0.0 {
            let eat_r2 = EAT_AGENT_RADIUS * EAT_AGENT_RADIUS;
            let mut edible_near = false;
            let mut best_target: Option<Vec2> = None;
            let mut best_d2: f32 = f32::INFINITY;
            for (j, (p, alive, consumed, _species, is_corpse)) in snapshot.iter().enumerate() {
                if j == idx { continue; }
                // edible if alive (predation) or dead but not yet consumed (scavenge)
                if (*alive && !PREDATION_ENABLED) || ((*is_corpse || !*alive) && !SCAVENGE_ENABLED) { continue; }
                if *consumed { continue; }
                let dx = p.x - a.pos.x; let dy = p.y - a.pos.y; let d2 = dx*dx + dy*dy;
                if d2 <= eat_r2 {
                    edible_near = true;
                    if d2 < best_d2 {
                        best_d2 = d2;
                        best_target = Some(*p);
                    }
                }
            }
            if edible_near {
                // Draw a directional line to the nearest edible target (orange), with a marker dot (no arrowheads)
                if let Some(tp) = best_target {
                    let (tx, ty) = world_to_screen(fitted, tp);
                    let col = Color::new(1.0, 0.6, 0.1, 0.95);
                    draw_line(px, py, tx, ty, 2.5, col);
                    draw_circle(tx, ty, 3.0, col);
                }
            }
        }
        // Predation flash: red ring
        if a.predation_flash_steps > 0 {
            draw_circle_lines(px, py, agent_r + 5.0, 3.0, Color::new(1.0, 0.1, 0.1, 0.95));
        }

        // Overlays for the focused agent
        if Some(idx) == focused_idx && a.energy > 0.0 {
            if unified_overlay {
                // Unified overlay: smoothed sector bars + memory vectors + density radial ticks
                // Draw sector bars using agent's smoothed pooled_* fields
                let (ax, ay) = world_to_screen(fitted, a.pos);
                let w_sector = 56.0; let bar_h = 7.0; let gap = 3.0;
                let colors = [Color::new(0.25,1.0,0.25,0.95), Color::new(0.1,0.85,1.0,0.95), Color::new(1.0,0.3,0.9,0.95), Color::new(0.75,0.75,0.75,0.95)];
                let rows: [[f32;3];4] = [a.pooled_plant, a.pooled_same, a.pooled_other, a.pooled_wall];
                for (sector_i, _) in ["L","F","R"].iter().enumerate() {
                    let x0 = ax - w_sector * 1.6 + sector_i as f32 * (w_sector + 16.0);
                    for (row_i, arr) in rows.iter().enumerate() {
                        let v = arr[sector_i].clamp(0.0,1.0);
                        let y0 = ay - 28.0 - (row_i as f32) * (bar_h + gap);
                        draw_rectangle(x0, y0, w_sector, bar_h, Color::new(0.07,0.08,0.1,0.7));
                        draw_rectangle(x0, y0, w_sector * v, bar_h, colors[row_i]);
                    }
                }
                // Hearing sector bars (magenta) single row below others
                for (sector_i, _) in ["L","F","R"].iter().enumerate() {
                    let x0 = ax - w_sector * 1.6 + sector_i as f32 * (w_sector + 16.0);
                    let hear_v = a.heard_sectors[sector_i].clamp(0.0, 1.0);
                    let y0 = ay - 28.0 - (4.0_f32) * (bar_h + gap); // one extra row beneath existing 4 rows
                    // background
                    draw_rectangle(x0, y0, w_sector, bar_h, Color::new(0.08,0.05,0.10,0.65));
                    // filled
                    draw_rectangle(x0, y0, w_sector * hear_v, bar_h, Color::new(0.95,0.3,1.0,0.9));
                }
                // Hearing label H to the right side
                let label_x = ax + w_sector * 1.6 + 10.0;
                let label_y = ay - 28.0 - (4.0_f32) * (bar_h + gap) + bar_h - 1.0;
                draw_text("H", label_x, label_y, 16.0, Color::new(0.95,0.3,1.0,0.9));
                // Memory vectors (food=yellow, danger=orange)
                let draw_mem_vec = |vx: f32, vy: f32, color: Color| {
                    let cth = a.theta.cos(); let sth = a.theta.sin();
                    let right_x = -sth; let right_y = cth; let fwd_x = cth; let fwd_y = sth;
                    let scale = 55.0;
                    let world_dx = (right_x * vx + fwd_x * vy) * (scale / fitted.w * WORLD_W);
                    let world_dy = (right_y * vx + fwd_y * vy) * (scale / fitted.h * WORLD_H);
                    let end = Vec2 { x: a.pos.x + world_dx, y: a.pos.y + world_dy };
                    let (ex, ey) = world_to_screen(fitted, end);
                    draw_line(ax, ay, ex, ey, 2.0, color);
                };
                draw_mem_vec(a.last_food_mem.x, a.last_food_mem.y, Color::new(1.0,0.95,0.3,0.9));
                draw_mem_vec(a.last_danger_mem.x, a.last_danger_mem.y, Color::new(1.0,0.6,0.2,0.9));
                // Density rays (scaled magnitude) using current snapshot
                let bins = sensing::density_sectors(a.pos, a.theta, &snapshot, idx);
                // Radial density lines (cyan) using bins array mapping: forward, side-L, side-R, back-L, back-R, back-center
                let dir_angles = [0.0, std::f32::consts::FRAC_PI_2*0.66, -std::f32::consts::FRAC_PI_2*0.66, std::f32::consts::PI*0.75, -std::f32::consts::PI*0.75, std::f32::consts::PI];
                for (bi, val) in bins.iter().enumerate() { if *val <= 0.0 { continue; }
                    let ang_world = a.theta + dir_angles[bi];
                    let len = (AGENT_RADIUS * 4.0) + *val * (AGENT_RADIUS * 6.0);
                    let end = Vec2 { x: a.pos.x + ang_world.cos() * len, y: a.pos.y + ang_world.sin() * len };
                    let (ex, ey) = world_to_screen(fitted, end);
                    draw_line(ax, ay, ex, ey, 2.0, Color::new(0.1,1.0,1.0,0.85));
                }
            }
        }
    }
}
