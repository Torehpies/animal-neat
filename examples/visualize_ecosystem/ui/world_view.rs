use macroquad::prelude::*;
use crate::params::*;
use crate::{Episode, Vec2};
use crate::sensing;
use crate::ui_common::{world_to_screen, fit_world_rect, world_scale};
use crate::dir_from_theta;

pub fn draw_world(area: Rect, episode: &Episode, show_cones: bool, _member_species: &[usize], show_density_overlay: bool, show_vector_overlay: bool, mouse_world: Option<Vec2>) {
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
    let snapshot: Vec<(Vec2, bool, bool)> = episode.agents.iter().map(|a| (a.pos, a.energy > 0.0, a.consumed)).collect();

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
            for (j, (p, alive, consumed)) in snapshot.iter().enumerate() {
                if j == idx { continue; }
                // edible if alive (predation) or dead but not yet consumed (scavenge)
                if (*alive && !PREDATION_ENABLED) || (!*alive && !SCAVENGE_ENABLED) { continue; }
                if !*alive && *consumed { continue; }
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
            if show_density_overlay {
                let bins = sensing::density_sectors(a.pos, a.theta, &snapshot, idx);
                let two_pi = std::f32::consts::PI * 2.0;
                let sector = two_pi / (DENSITY_SECTORS as f32);
                let base_len = 18.0_f32.max(agent_r + 4.0);
                for s in 0..DENSITY_SECTORS {
                    let v = bins[s].clamp(0.0, 1.0);
                    if v <= 0.0 { continue; }
                    // mid-angle of sector in local frame: 0 = right, +pi/2 = forward
                    let ang_local = -std::f32::consts::PI + sector * (s as f32 + 0.5);
                    // Convert local dir to world delta with scale
                    let c = a.theta.cos(); let snt = a.theta.sin();
                    let right_x = -snt; let right_y = c;
                    let fwd_x = c; let fwd_y = snt;
                    let dir_world_x = right_x * ang_local.cos() + fwd_x * ang_local.sin();
                    let dir_world_y = right_y * ang_local.cos() + fwd_y * ang_local.sin();
                    let len = base_len + v * 28.0;
                    let end_world = Vec2 { x: a.pos.x + dir_world_x * (len / fitted.w * WORLD_W), y: a.pos.y + dir_world_y * (len / fitted.h * WORLD_H) };
                    let (ex, ey) = world_to_screen(fitted, end_world);
                    draw_line(px, py, ex, ey, 2.0, Color::new(0.1, 1.0, 1.0, 0.8));
                }
            }
            if show_vector_overlay {
                // Helper to draw an arrow for a local vector
                let draw_local_arrow = |_label: &str, lx: f32, ly: f32, color: Color| {
                    let c = a.theta.cos(); let snt = a.theta.sin();
                    let right_x = -snt; let right_y = c;
                    let fwd_x = c; let fwd_y = snt;
                    let scale = 60.0; // pixels
                    let world_dx = (right_x * lx + fwd_x * ly) * (scale / fitted.w * WORLD_W);
                    let world_dy = (right_y * lx + fwd_y * ly) * (scale / fitted.h * WORLD_H);
                    let end = Vec2 { x: a.pos.x + world_dx, y: a.pos.y + world_dy };
                    let (ex, ey) = world_to_screen(fitted, end);
                    draw_line(px, py, ex, ey, 2.0, color);
                    // arrow head
                    let hx = ex + (px - ex) * 0.15 + (ey - py) * 0.12;
                    let hy = ey + (py - ey) * 0.15 - (ex - px) * 0.12;
                    draw_line(ex, ey, hx, hy, 2.0, color);
                    let hx2 = ex + (px - ex) * 0.15 - (ey - py) * 0.12;
                    let hy2 = ey + (py - ey) * 0.15 + (ex - px) * 0.12;
                    draw_line(ex, ey, hx2, hy2, 2.0, color);
                };
                // Current food vector (green) derived from rays
                let (fx, fy) = sensing::food_vector_from_rays(a.pos, a.theta, &episode.food);
                draw_local_arrow("food", fx, fy, Color::new(0.2, 1.0, 0.2, 0.95));
                // Current meat vector (orange) derived from rays (edible agents or corpses)
                let (mx, my) = sensing::meat_vector_from_rays(a.pos, a.theta, &snapshot, idx);
                draw_local_arrow("meat", mx, my, Color::new(1.0, 0.6, 0.1, 0.95));
                // Current danger vector (red)
                let (dx, dy) = sensing::nearest_agent_vector_local(a.pos, a.theta, &snapshot, idx);
                draw_local_arrow("danger", dx, dy, Color::new(1.0, 0.2, 0.2, 0.9));
                // Memory vectors (yellow/orange)
                draw_local_arrow("last_food", a.last_food_mem.x, a.last_food_mem.y, Color::new(1.0, 0.9, 0.2, 0.95));
                draw_local_arrow("last_danger", a.last_danger_mem.x, a.last_danger_mem.y, Color::new(1.0, 0.6, 0.2, 0.95));
            }
        }
    }
}
