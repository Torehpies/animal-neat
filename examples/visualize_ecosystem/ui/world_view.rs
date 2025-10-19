use macroquad::prelude::*;
use crate::params::*;
use crate::{sim::Episode, Vec2};
use crate::sensing;
use crate::ui_common::{world_to_screen, fit_world_rect, world_scale};
use crate::sim::{dir_from_theta, AgentKind};

pub fn draw_world(
    area: Rect,
    episode: &Episode,
    show_cones: bool,
    _member_species: &[usize],
    unified_overlay: bool,
    mouse_world: Option<Vec2>,
    show_energy_overlay: bool,
    show_collision_radii: bool,
    focused_agent: Option<usize>,
    show_grid: bool,
    show_vis_inputs: bool,
    show_hearing_inputs: bool,
    show_memory_inputs: bool,
    color_by_species: bool,
    herb_tex: Option<&Texture2D>,
    carn_tex: Option<&Texture2D>,
    plant_tex: Option<&Texture2D>,
    meat_tex: Option<&Texture2D>,
) {
    let fitted = fit_world_rect(area);
    let world_w = crate::world::get_world_w();
    let world_h = crate::world::get_world_h();
    // background: draw biome bands with seasonal tinting
    // Biomes split across X using BIOME_X_SPLITS; use episode.steps as season time
    let splits = BIOME_X_SPLITS;
    let bands = [0.0, splits[0], splits[1], 1.0];
    for b in 0..3 {
        let x0w = bands[b] * world_w;
        let x1w = bands[b + 1] * world_w;
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
    // exploration grid overlay
    if show_grid {
        let nx = (world_w / EXPL_CELL_SIZE).ceil() as i32;
        let ny = (world_h / EXPL_CELL_SIZE).ceil() as i32;
        // Vertical lines
        for i in 0..=nx {
            let xw = if i >= nx { world_w } else { i as f32 * EXPL_CELL_SIZE };
            let (x0, y0) = world_to_screen(fitted, Vec2 { x: xw, y: 0.0 });
            let (x1, y1) = world_to_screen(fitted, Vec2 { x: xw, y: world_h });
            let major = i % 5 == 0;
            let col = if major { Color::new(1.0, 1.0, 1.0, 0.28) } else { Color::new(1.0, 1.0, 1.0, 0.12) };
            let w = if major { 2.0 } else { 1.0 };
            draw_line(x0, y0, x1, y1, w, col);
        }
        // Horizontal lines
        for j in 0..=ny {
            let yw = if j >= ny { world_h } else { j as f32 * EXPL_CELL_SIZE };
            let (x0, y0) = world_to_screen(fitted, Vec2 { x: 0.0, y: yw });
            let (x1, y1) = world_to_screen(fitted, Vec2 { x: world_w, y: yw });
            let major = j % 5 == 0;
            let col = if major { Color::new(1.0, 1.0, 1.0, 0.28) } else { Color::new(1.0, 1.0, 1.0, 0.12) };
            let w = if major { 2.0 } else { 1.0 };
            draw_line(x0, y0, x1, y1, w, col);
        }
        // Label with cell size
        let pad = 6.0;
        let label = format!("Grid {:.0}×{:.0}", EXPL_CELL_SIZE, EXPL_CELL_SIZE);
        let tw = measure_text(&label, None, 16, 1.0).width;
        let th = 16.0;
        let bx = fitted.x + pad;
        let by = fitted.y + pad;
        draw_rectangle(bx - 4.0, by - th + 2.0, tw + 10.0, th + 6.0, Color::new(0.05, 0.05, 0.08, 0.7));
        draw_text(&label, bx, by + 2.0, 16.0, WHITE);
    }
    // food
    for p in &episode.food {
        let (px, py) = world_to_screen(fitted, *p);
        let r = (FOOD_RADIUS * px_per_world).max(2.0);
        // Draw plant sprite if available, otherwise fallback to circle
        if let Some(tex) = plant_tex {
            let size = r * 2.0;
            let params = DrawTextureParams { dest_size: Some(vec2(size, size)), ..Default::default() };
            draw_texture_ex(tex, px - size * 0.5, py - size * 0.5, WHITE, params);
        } else {
            draw_circle(px, py, r, YELLOW);
        }
        if show_collision_radii {
            // High-contrast collision radius for food (white ring)
            draw_circle_lines(px, py, (FOOD_RADIUS * px_per_world).max(1.0), 2.0, Color::new(1.0, 1.0, 1.0, 0.95));
        }
    }
    // Precompute snapshot for overlays
    let snapshot: Vec<(Vec2, bool, bool, usize, bool)> = episode.agents.iter().map(|a| {
        let alive = a.energy > 0.0;
        let is_corpse = !alive && !a.consumed && a.corpse_energy > 0.1;
        (a.body.pos, alive, a.consumed, a.species_id, is_corpse)
    }).collect();

    // Determine focused agent (nearest to mouse)
    let focused_idx: Option<usize> = mouse_world.and_then(|mw| {
        let mut best: Option<(usize, f32)> = None;
        for (i, a) in episode.agents.iter().enumerate() {
            if a.energy <= 0.0 { continue; }
            let dx = a.body.pos.x - mw.x; let dy = a.body.pos.y - mw.y; let d2 = dx*dx + dy*dy;
            if let Some((_, b)) = best { if d2 < b { best = Some((i, d2)); } } else { best = Some((i, d2)); }
        }
        best.map(|(i, _)| i)
    });

    // agents
    for (idx, a) in episode.agents.iter().enumerate() {
    let (px, py) = world_to_screen(fitted, a.body.pos);
    // Visual radius scaled for rendering only; collisions still use AGENT_RADIUS in world units elsewhere
    let agent_r = (AGENT_RADIUS * AGENT_RENDER_SCALE * px_per_world).max(3.0);
    // species index unused for coloring now that diet-based coloring is applied
        // draw alive vs dead differently
        if a.energy > 0.0 && a.health > DEATH_HEALTH_THRESHOLD {
            // Try sprite first if available; otherwise fall back to colored circle
            let tex_opt: Option<&Texture2D> = match a.kind {
                AgentKind::Herbivore => herb_tex,
                AgentKind::Carnivore => carn_tex,
            };
            if let Some(tex) = tex_opt {
                // Draw the sprite centered at (px,py) and flip horizontally if moving/facing right.
                let size = agent_r * 2.0;
                // Determine facing from velocity primary, fallback to heading when nearly stationary
                let facing_right = if a.body.vel.x.abs() > 0.05 { a.body.vel.x > 0.0 } else { a.theta.cos() > 0.0 };
                let params = DrawTextureParams {
                    dest_size: Some(vec2(size, size)),
                    flip_x: facing_right,
                    ..Default::default()
                };
                draw_texture_ex(tex, px - size * 0.5, py - size * 0.5, WHITE, params);
                // Small center dot to verify anchor alignment
                draw_circle(px, py, 1.5, BLACK);
            } else {
                // Color either by species or by diet, based on toggle
                let fill = if color_by_species {
                    let sidx = a.species_id as u32;
                    let hue = ((sidx % 12) as f32) / 12.0; // 12 distinct hues cycling
                    let (r, g, b) = crate::ui_common::hsv_to_rgb(hue, 0.85, 0.95);
                    let hf = (a.health / a.max_health).clamp(0.0,1.0);
                    Color::new(r * (0.5 + 0.5*hf), g * (0.5 + 0.5*hf), b * (0.5 + 0.5*hf), 1.0)
                } else {
                    // Fixed tint by agent kind: herbivore green, carnivore red
                    let (r, g, b) = match a.kind {
                        AgentKind::Herbivore => crate::ui_common::hsv_to_rgb(1.0/3.0, 0.85, 0.95),
                        AgentKind::Carnivore => crate::ui_common::hsv_to_rgb(0.0, 0.85, 0.95),
                    };
                    let hf = (a.health / a.max_health).clamp(0.0,1.0);
                    Color::new(r * (0.5 + 0.5*hf), g * (0.5 + 0.5*hf), b * (0.5 + 0.5*hf), 1.0)
                };
                draw_circle(px, py, agent_r, fill);
            }
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
            // Draw meat sprite for corpse if available; otherwise fallback to gray circle
            if let Some(meat) = meat_tex {
                let size = agent_r * 2.0;
                let params = DrawTextureParams { dest_size: Some(vec2(size, size)), ..Default::default() };
                draw_texture_ex(meat, px - size * 0.5, py - size * 0.5, WHITE, params);
            } else {
                let fill = Color::new(0.25, 0.25, 0.25, 0.9);
                draw_circle(px, py, agent_r, fill);
            }
        }
    // outline disabled - was: draw_circle_lines(px, py, agent_r, 2.0, Color::new(0.2, 0.2, 0.2, 0.6));
        if show_collision_radii {
            // High-contrast collision radius for agents (white ring)
            draw_circle_lines(px, py, (AGENT_COLLISION_RADIUS * px_per_world).max(1.0), 2.0, Color::new(1.0, 1.0, 1.0, 0.95));
        }
        // heading line
        if a.energy > 0.0 {
            let dir = dir_from_theta(a.theta);
            let (hx, hy) = world_to_screen(fitted, Vec2 { x: a.body.pos.x + dir.x * 2.0, y: a.body.pos.y + dir.y * 2.0 });
            draw_line(px, py, hx, hy, 2.0, BLUE);
        }

        if show_cones && a.energy > 0.0 {
            // Visualize mating radius: faint ring showing ECO_MATE_RADIUS for pair reproduction
            let mate_r_px = (ECO_MATE_RADIUS * px_per_world).max(1.0);
            draw_circle_lines(px, py, mate_r_px, 1.5, Color::new(0.95, 0.3, 1.0, 0.35));

            let dir = dir_from_theta(a.theta);
            for r in sensing::ray_directions(dir) {
                let food_t = sensing::nearest_food_along_ray(a.body.pos, r, &episode.food);
                match food_t {
                    Some(t) => {
                        let sense_pt = Vec2 { x: a.body.pos.x + r.x * t, y: a.body.pos.y + r.y * t };
                        let (sx, sy) = world_to_screen(fitted, sense_pt);
                        // draw sensed segment in green up to the food point
                        let green = Color::new(0.2, 1.0, 0.2, 0.9);
                        draw_line(px, py, sx, sy, 2.0, green);
                        // mark the sensed point
                        draw_circle(sx, sy, 3.0, green);
                        // faint remainder to max range (lighter green)
                        let end = Vec2 { x: a.body.pos.x + r.x * VISION_RANGE, y: a.body.pos.y + r.y * VISION_RANGE };
                        let (x2, y2) = world_to_screen(fitted, end);
                        draw_line(sx, sy, x2, y2, 1.0, Color::new(0.2, 1.0, 0.2, 0.25));
                    }
                    None => {
                        let end = Vec2 { x: a.body.pos.x + r.x * VISION_RANGE, y: a.body.pos.y * 1.0 + r.y * VISION_RANGE };
                        let (x2, y2) = world_to_screen(fitted, end);
                        draw_line(px, py, x2, y2, 1.0, Color::new(0.2, 1.0, 0.2, 0.35));
                    }
                }
                // Overlay of other categories removed for simplicity in cone view; use inputs grid below for full breakdown.
            }
        }
        // Visual cue: edible nearby (live prey or unconsumed corpse) within EAT_AGENT_RADIUS
        if a.energy > 0.0 {
            // Visualize edible radius using collision-scaled eat radius
            let eat_collision_radius = EAT_AGENT_RADIUS * AGENT_RENDER_SCALE;
            let eat_r2 = eat_collision_radius * eat_collision_radius;
            let mut edible_near = false;
            let mut best_target: Option<Vec2> = None;
            let mut best_d2: f32 = f32::INFINITY;
            for (j, (p, alive, consumed, _species, is_corpse)) in snapshot.iter().enumerate() {
                if j == idx { continue; }
                // edible if alive (predation) or dead but not yet consumed (scavenge)
                if (*alive && !PREDATION_ENABLED) || ((*is_corpse || !*alive) && !SCAVENGE_ENABLED) { continue; }
                if *consumed { continue; }
                let dx = p.x - a.body.pos.x; let dy = p.y - a.body.pos.y; let d2 = dx*dx + dy*dy;
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
        // Birth flash: bright expanding ring for a few frames after birth
        if a.age_steps < NEWBORN_FLASH_STEPS {
            let t = a.age_steps as f32 / (NEWBORN_FLASH_STEPS as f32).max(1.0);
            let ring_r = agent_r + 4.0 + t * 18.0;
            let alpha = 0.75 * (1.0 - t);
            draw_circle_lines(px, py, ring_r, 3.0, Color::new(1.0, 1.0, 0.2, alpha));
        }
        // Newborn grace halo removed for realism; keep only brief birth flash above

        // Focus highlight (draw after other rings for visibility)
        if Some(idx) == focused_agent {
            draw_circle_lines(px, py, agent_r + 8.0, 3.0, Color::new(0.25, 0.9, 1.0, 0.95));
            draw_circle_lines(px, py, agent_r + 11.5, 2.0, Color::new(0.25, 0.9, 1.0, 0.55));
        }

        // Overlays for the focused agent
        if Some(idx) == focused_idx && a.energy > 0.0 {
            // Basic energy overlay (always shown when hovering) -- draw first
            if show_energy_overlay {
            let energy_frac = (a.energy / crate::params::get_max_energy()).clamp(0.0, 1.0);
            let bar_w = 70.0; let bar_h = 7.0; let pad = 3.0;
            let bx = px - bar_w * 0.5; let by = py - agent_r - 18.0;
            // background box
            draw_rectangle(bx - pad, by - pad - 10.0, bar_w + pad * 2.0, bar_h + pad * 2.0 + 10.0, Color::new(0.05,0.05,0.08,0.80));
            // bar background
            draw_rectangle(bx, by, bar_w, bar_h, Color::new(0.15,0.15,0.2,0.9));
            // bar fill (gradient-ish: lerp red->yellow->green via fraction)
            let (r,g,b) = if energy_frac < 0.5 {
                // red (low) to yellow (mid)
                let t = energy_frac / 0.5; (1.0, 0.2 + 0.6*t, 0.1)
            } else {
                // yellow to green
                let t = (energy_frac - 0.5) / 0.5; (1.0 - 0.5*t, 0.8 + 0.2*t, 0.1 + 0.4*t)
            };
            draw_rectangle(bx, by, bar_w * energy_frac, bar_h, Color::new(r,g,b,0.95));
            let energy_text = format!("E: {:.0}/{:.0}", a.energy.max(0.0), crate::params::get_max_energy());
            draw_text(&energy_text, bx, by - 2.0, 14.0, WHITE);
            }
            if unified_overlay {
                // Unified overlay: smoothed sector bars + memory vectors + density radial ticks
                let (ax, ay) = world_to_screen(fitted, a.body.pos);
                let w_sector = 56.0; let bar_h = 7.0; let gap = 3.0;
                // Vision overlay for pooled inputs removed (revised vision model uses distances)
                if show_hearing_inputs && HEARING_SECTORS > 0 {
                    for (sector_i, _) in ["L","F","R"].iter().enumerate() {
                        let x0 = ax - w_sector * 1.6 + sector_i as f32 * (w_sector + 16.0);
                        let hear_v = a.heard_sectors[sector_i].clamp(0.0, 1.0);
                        let y0 = ay - 28.0 - (4.0_f32) * (bar_h + gap);
                        draw_rectangle(x0, y0, w_sector, bar_h, Color::new(0.08,0.05,0.10,0.65));
                        draw_rectangle(x0, y0, w_sector * hear_v, bar_h, Color::new(0.95,0.3,1.0,0.9));
                    }
                    let label_x = ax + w_sector * 1.6 + 10.0;
                    let label_y = ay - 28.0 - (4.0_f32) * (bar_h + gap) + bar_h - 1.0;
                    draw_text("H", label_x, label_y, 16.0, Color::new(0.95,0.3,1.0,0.9));
                }
                if show_vis_inputs {
                    // Draw a rays×categories grid encoding (1 - distance)
                    // Category order per ray: Plant, Carcass, Same, Other, Wall
                    let grid_w = 10.0; let grid_h = 8.0; let pad = 2.0;
                    let total_cols = VISION_RAYS as i32; let total_rows = 5i32;
                    let base_x = ax - (grid_w + pad) * total_cols as f32 * 0.5;
                    let base_y = ay - agent_r - 80.0; // stack above energy bar
                    let labels = ["P","C","S","O","W"]; // left side labels
                    let inputs = sensing::build_inputs(
                        a.body.pos,
                        a.theta,
                        &episode.food,
                        (a.energy / crate::params::get_max_energy()).clamp(0.0,1.0),
                        a.last_food_mem,
                        a.last_same_mem,
                        a.last_other_mem,
                        &snapshot,
                        idx,
                        a.species_id,
                        a.heard_sectors,
                    );
                    let ranges = sensing::input_ranges();
                    for row in 0..total_rows {
                        for col in 0..total_cols {
                            let base = ranges.vision.start + (col as usize) * 5;
                            let dist_norm = inputs[base + (row as usize)];
                            let prox = (1.0 - dist_norm).clamp(0.0, 1.0);
                            let x0 = base_x + col as f32 * (grid_w + pad);
                            let y0 = base_y + row as f32 * (grid_h + pad);
                            draw_rectangle(x0, y0, grid_w, grid_h, Color::new(0.05,0.06,0.08,0.85));
                            let colr = match row { 0 => Color::new(0.2,1.0,0.2,0.95), // plant
                                                   1 => Color::new(0.9,0.65,0.2,0.95), // carcass
                                                   2 => Color::new(0.1,0.85,1.0,0.95), // same
                                                   3 => Color::new(1.0,0.3,0.95,0.95), // other
                                                   _ => Color::new(0.75,0.75,0.75,0.95) }; // wall
                            if prox > 0.0 { draw_rectangle(x0, y0, grid_w * prox, grid_h, colr); }
                        }
                        let lx = base_x - 18.0;
                        let ly = base_y + row as f32 * (grid_h + pad) + grid_h - 2.0;
                        draw_text(labels[row as usize], lx, ly, 14.0, GRAY);
                    }
                }
                // Memory vectors (food=yellow, danger=orange)
                if show_memory_inputs {
                let draw_mem_vec = |vx: f32, vy: f32, color: Color| {
                    let world_w = crate::world::get_world_w();
                    let world_h = crate::world::get_world_h();
                    let cth = a.theta.cos(); let sth = a.theta.sin();
                    let right_x = -sth; let right_y = cth; let fwd_x = cth; let fwd_y = sth;
                    let scale = 55.0;
                    let world_dx = (right_x * vx + fwd_x * vy) * (scale / fitted.w * world_w);
                    let world_dy = (right_y * vx + fwd_y * vy) * (scale / fitted.h * world_h);
                    let end = Vec2 { x: a.body.pos.x + world_dx, y: a.body.pos.y + world_dy };
                    let (ex, ey) = world_to_screen(fitted, end);
                    draw_line(ax, ay, ex, ey, 2.0, color);
                };
                draw_mem_vec(a.last_food_mem.x, a.last_food_mem.y, Color::new(1.0,0.95,0.3,0.9));
                draw_mem_vec(a.last_same_mem.x, a.last_same_mem.y, Color::new(0.3,0.9,1.0,0.9));
                draw_mem_vec(a.last_other_mem.x, a.last_other_mem.y, Color::new(1.0,0.6,0.2,0.9));
                }
                // Density visualization removed with revised sensing model
            }
        }
    }
}
