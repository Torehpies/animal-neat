use macroquad::prelude::*;
use crate::AppState;
use crate::params::*;
use crate::ui_common::{
    draw_text_clamped, draw_text_wrapped, draw_panel, section_title, draw_divider,
    PANEL_BG, PANEL_BORDER, SUBPANEL_BG, PAD, GAP, FONT,
};
use crate::ui_network::{draw_network_panel};
use crate::ui_graphs::draw_graphs_panel;

// Compact HUD with essential stats; best-network panel retained.
pub fn draw_hud(area: Rect, state: &mut AppState, running: &mut bool, fast_mode: &mut bool) {
    // Main panel
    draw_panel(area, PANEL_BG, PANEL_BORDER, 2.0);

    let x = area.x + PAD;
    let mut y = area.y + PAD;

    // Reserve bottom portion for network panel; can be hidden via toggles
    // Graphs are now shown in a separate overlay; reserve no HUD space
    let graphs_h = 0.0;
    let network_h = if state.show_best_network_panel { (area.h * 0.542).clamp(160.0, 380.0) } else { 0.0 };
    let reserved_h = graphs_h + network_h;
    let max_y = area.y + area.h - PAD - reserved_h - 8.0;
    let max_w = area.w - (x - area.x) - PAD;

    // Derived stats
    // species_count (NEAT speciator) removed: HUD should not show NEAT species
    let alive = state.episode.agents.iter().filter(|a| a.energy > 0.0).count();
    let corpses = state.episode.agents.iter().filter(|a| a.energy <= 0.0 && !a.consumed).count();
    // Per-species alive counts
    let herb_alive = state
        .episode
        .agents
        .iter()
        .filter(|a| a.kind == crate::sim::AgentKind::Herbivore && a.energy > 0.0)
        .count();
    let carn_alive = state
        .episode
        .agents
        .iter()
        .filter(|a| a.kind == crate::sim::AgentKind::Carnivore && a.energy > 0.0)
        .count();
    let (min_e, avg_e, max_e) = if !state.episode.agents.is_empty() {
        let mut min_e = f32::INFINITY; let mut max_e = f32::NEG_INFINITY; let mut sum = 0.0;
        for a in &state.episode.agents { min_e = min_e.min(a.energy); max_e = max_e.max(a.energy); sum += a.energy; }
        (min_e, sum / state.episode.agents.len() as f32, max_e)
    } else { (0.0, 0.0, 0.0) };

    // Top-right: FPS pill (always shown)
    {
        let fs = 14.0;
        let fps = get_fps();
        let fps_txt = format!("{} fps", fps);
        let pad_x = 8.0; let pad_y = 4.0;
        let dims2 = measure_text(&fps_txt, None, fs as u16, 1.0);
        let pill_w2 = dims2.width + 2.0 * pad_x;
        let pill_h2 = fs + 2.0 * pad_y;
        let px2 = area.x + area.w - PAD - pill_w2;
        let py2 = area.y + 6.0;
        draw_rectangle(px2, py2, pill_w2, pill_h2, Color::new(0.15, 0.18, 0.22, 0.25));
        draw_rectangle_lines(px2, py2, pill_w2, pill_h2, 1.0, Color::new(0.45, 0.55, 0.70, 0.55));
        draw_text(&fps_txt, px2 + pad_x, py2 + fs, fs, Color::new(0.85, 0.9, 0.95, 1.0));
    }

    // Essentials block
    y = section_title("Overview", x, y, max_w);
    let essentials = [
        if ECO_CONTINUOUS {
            format!("ECO ep {}", state.eco_episode_counter)
        } else {
            format!("Gen {}", state.generation)
        },
    format!("Pop {}", state.population.len()),
    format!("Herb {} | Carn {}", herb_alive, carn_alive),
        format!("Best {:.2} | Avg {:.2}", state.last_best, state.last_avg),
        format!("Steps {} | Alive {}", state.episode.steps, alive),
        if ECO_CONTINUOUS { format!("Births this ep: {}", state.episode.births_this_episode) } else { format!("Corpses {}", corpses) },
        if ECO_CONTINUOUS { format!("Corpses {}", corpses) } else { format!("Energy min/avg/max: {:.0}/{:.0}/{:.0}", min_e, avg_e, max_e) },
        format!("Inputs: {}", INPUTS),
    ];
    for line in essentials.iter() {
        if y > max_y { break; }
        y = draw_text_wrapped(line, x, y, FONT, WHITE, max_w, GAP);
    }
    y = draw_divider(x, y, max_w);

    // Focused agent detail block (if any)
    if y <= max_y {
        if let Some(fi) = state.focused_agent {
            if let Some(a) = state.episode.agents.get(fi) {
                let hdr = format!("Focused Agent #{}", fi);
                y = section_title(&hdr, x, y, max_w);
                let alive = a.energy > 0.0 && a.health > 0.0;
                let plants_eaten = a.eaten.saturating_sub(a.kills);
                let stats_line = match a.kind { 
                    crate::sim::AgentKind::Herbivore => format!("Plants eaten {} | CorpseEnergy {:.0}", plants_eaten, a.corpse_energy),
                    crate::sim::AgentKind::Carnivore => format!("Kills {} | CorpseEnergy {:.0}", a.kills, a.corpse_energy),
                };
                let lines = [
                    // Don't display NEAT species id in HUD; show births only
                    format!("Births {}", a.offspring_count),
                    format!("Type: {}", match a.kind { crate::sim::AgentKind::Herbivore => "Herbivore", crate::sim::AgentKind::Carnivore => "Carnivore" }),
                    format!("Status: {}", if alive { "Alive" } else { "Dead" }),
                    format!("Energy {:.0}/{:.0}", a.energy.max(0.0), crate::params::get_max_energy()),
                    format!("Health {:.0}/{:.0}", a.health.max(0.0), a.max_health),
                    format!("Alive steps {}", a.alive_steps),
                    stats_line,
                ];
                for line in lines.iter() { if y > max_y { break; } y = draw_text_wrapped(line, x+4.0, y, 16.0, GRAY, max_w, 4.0); }
            }
        }
    }

    // Controls (toggleable) with two columns, one control per row, short descriptions
    if state.show_controls {
        if y <= max_y {
            y = section_title("Controls", x, y, max_w);
            // Simple square indicator for toggles: filled when ON, outline when OFF
            let ind_on = Color::new(0.95, 0.8, 0.25, 1.0);
            let ind_off = Color::new(0.45, 0.5, 0.6, 1.0);
            let col_gap = 12.0;
            let col_w = (max_w - col_gap) * 0.5;
            let fs = 17.0;
            let ind_size = 9.0;
            let ind_pad = 4.0;
            let key_pad = 0.0; // no extra spacing between key and description

            // Left column rows
            let mut yl = y;
            let left_rows: &[( &str, &str, bool )] = &[
                ("[R]", "Reset Episode", false),
                ("[Esc]", "Clear Focus", state.focused_agent.is_some()),
                ("[CLK]", "Focus Agent", state.focused_agent.is_some()),
                ("[N]", "Best Network", state.show_best_network_panel),
                ("[H]", "Toggle Controls", state.show_controls),
            ];
            // Compute dynamic key slot width for left column
            let mut key_slot_w_left = 0.0f32;
            for (key, _, _) in left_rows.iter() {
                let w = measure_text(key, None, fs as u16, 1.0).width;
                if w > key_slot_w_left { key_slot_w_left = w; }
            }
            key_slot_w_left += 1.0; // minimal padding
            // clamp to a tighter range so the gap doesn't look excessive
            key_slot_w_left = key_slot_w_left.clamp(24.0, 44.0);
            for (key, desc, on) in left_rows.iter() {
                if yl > max_y { break; }
                // indicator box
                let ix = x + 6.0;
                let iy = yl - fs + (fs - ind_size) * 0.5; // center square to text row
                draw_rectangle(ix, iy, ind_size, ind_size, if *on { ind_on } else { Color::new(0.0,0.0,0.0,0.0) });
                draw_rectangle_lines(ix, iy, ind_size, ind_size, 1.2, if *on { ind_on } else { ind_off });
                // draw key in fixed slot
                let key_x = ix + ind_size + ind_pad;
                draw_text_clamped(key, key_x, yl, fs, LIGHTGRAY, key_slot_w_left);
                // draw desc aligned to constant x regardless of key width
                let desc_x = key_x + key_slot_w_left + key_pad;
                let desc_w = col_w - (desc_x - (x + 6.0));
                draw_text_clamped(desc, desc_x, yl, fs, LIGHTGRAY, desc_w);
                yl += fs + 6.0;
            }

            // Right column rows
            let mut yr = y;
            let right_rows: &[( &str, &str, bool )] = &[
                ("[T]", "Pause at episode end", state.show_scoreboard_panel),
                ("[C]", if state.scoreboard_pending { "Continue" } else { "Collision Radii" }, if state.scoreboard_pending { false } else { state.show_collision_radii }),
                ("[V]", "Vision Rays", state.show_cones),
                ("[U]", "Unified Overlay", state.show_unified_overlay),
                ("[E]", "Energy Bar", state.show_energy_overlay),
                ("[G]", "Exploration Grid", state.show_grid),
                ("[Z]", "Graphs Panel", state.show_graphs_panel),
                // FPS counter is always displayed now; removed toggle row
                ("[S]", "Quick Save (HUD)", false),
                ("[L]", "Load Latest", false),
                ("[K]", "Color by Species", state.color_by_species),
            ];
            // Compute dynamic key slot width for right column
            let mut key_slot_w_right = 0.0f32;
            for (key, _, _) in right_rows.iter() {
                let w = measure_text(key, None, fs as u16, 1.0).width;
                if w > key_slot_w_right { key_slot_w_right = w; }
            }
            key_slot_w_right += 1.0;
            key_slot_w_right = key_slot_w_right.clamp(24.0, 32.0);
            for (key, desc, on) in right_rows.iter() {
                if yr > max_y { break; }
                let ix = x + 6.0 + col_w + col_gap;
                let iy = yr - fs + (fs - ind_size) * 0.5;
                draw_rectangle(ix, iy, ind_size, ind_size, if *on { ind_on } else { Color::new(0.0,0.0,0.0,0.0) });
                draw_rectangle_lines(ix, iy, ind_size, ind_size, 1.2, if *on { ind_on } else { ind_off });
                // key in fixed slot
                let key_x = ix + ind_size + ind_pad;
                draw_text_clamped(key, key_x, yr, fs, LIGHTGRAY, key_slot_w_right);
                // desc aligned to constant x
                let desc_x = key_x + key_slot_w_right + key_pad;
                let desc_w = col_w - (desc_x - (x + 6.0 + col_w + col_gap));
                draw_text_clamped(desc, desc_x, yr, fs, LIGHTGRAY, desc_w);
                yr += fs + 6.0;
            }
            let _y_end = yl.max(yr) + GAP;
        }
    } else {
        // Compact hint when controls are hidden
        if y <= max_y {
            let hint = "[H] Show Controls";
            let _ = draw_text_wrapped(hint, x, y, 16.0, GRAY, max_w, GAP);
        }
    }

    // Bottom-left: vertical stacked toggle buttons (top-to-bottom): Ultra, Fast, Pause
    {
        let h = screen_height();
        let btn_h = 34.0;
        let spacing = 8.0;
        let labels = ["Ultra", "Fast", "Pause"];
        let states = [state.ultra_mode, *fast_mode, !*running];
        let start_x = 16.0;
        // compute equal width based on widest label
        let mut max_tw = 0.0f32;
        for l in labels.iter() {
            let w = measure_text(l, None, 16u16, 1.0).width;
            if w > max_tw { max_tw = w; }
        }
        let btn_padding_x = 14.0;
        let btn_w = max_tw + btn_padding_x * 2.0;
        // compute starting y so the stack sits above the bottom margin
        let total_h = labels.len() as f32 * btn_h + (labels.len() as f32 - 1.0) * spacing;
        let start_y = h - total_h - 18.0;
        for (i, label) in labels.iter().enumerate() {
            let cx = start_x;
            let cy = start_y + i as f32 * (btn_h + spacing);
            let (mx, my) = mouse_position();
            let hovering = mx >= cx && mx <= cx + btn_w && my >= cy && my <= cy + btn_h;
            let on = states[i];
            let bg = if on { Color::new(0.22, 0.58, 0.95, 1.0) } else if hovering { Color::new(0.18, 0.18, 0.18, 1.0) } else { Color::new(0.12, 0.12, 0.12, 0.9) };
            draw_rectangle(cx, cy, btn_w, btn_h, bg);
            draw_rectangle_lines(cx, cy, btn_w, btn_h, 1.0, Color::new(0.6,0.6,0.6,0.8));
            draw_text(label, cx + btn_padding_x, cy + (btn_h * 0.65), 16.0, WHITE);
            if hovering && is_mouse_button_pressed(MouseButton::Left) {
                match i {
                    0 => state.ultra_mode = !state.ultra_mode,
                    1 => *fast_mode = !*fast_mode,
                    2 => *running = !*running,
                    _ => {}
                }
            }
        }
    }

    // Graphs panel area (trends)
    if state.show_graphs_panel && graphs_h > 0.0 {
        let panel = Rect { x: area.x + 8.0, y: area.y + area.h - (network_h + graphs_h) + 8.0, w: area.w - 16.0, h: graphs_h - 16.0 };
        draw_graphs_panel(panel, &state.graphs);
    }

    // Network panel area (for best or live activations)
    if state.show_best_network_panel && network_h > 0.0 {
        let panel = Rect { x: area.x + 8.0, y: area.y + area.h - network_h + 8.0, w: area.w - 16.0, h: network_h - 16.0 };
        let frame = Rect { x: panel.x - 4.0, y: panel.y - 4.0, w: panel.w + 8.0, h: panel.h + 8.0 };
        draw_panel(frame, SUBPANEL_BG, PANEL_BORDER, 2.0);
        if state.show_best_network_panel {
            // If an agent is focused, prefer showing its network in the panel so clicking an agent
            // displays that agent's network (keeps the visual layout unchanged).
            if let Some(fi) = state.focused_agent {
                if fi < state.population.len() {
                    let title = format!("Focused Agent #{} network", fi);
                    draw_text_clamped(&title, panel.x, panel.y - 8.0, 18.0, LIGHTGRAY, panel.w - 8.0);
                    let genome = &state.population[fi];
                    draw_network_panel(panel, genome);
                } else {
                    // Fallback to best network if focused index is out-of-range
                    let title = if state.last_best.is_finite() && state.last_best > f32::NEG_INFINITY {
                        if ECO_CONTINUOUS { format!("Best network (last eval, fit {:.2})", state.last_best) } else { format!("Best network (last gen {}, fit {:.2})", state.last_best_generation, state.last_best) }
                    } else { "Best network (pending)".to_string() };
                    draw_text_clamped(&title, panel.x, panel.y - 8.0, 18.0, LIGHTGRAY, panel.w - 8.0);
                    if let Some(genome) = state.last_best_genome.as_ref() {
                        draw_network_panel(panel, genome);
                    } else {
                        let msg = if ECO_CONTINUOUS { "Evolves with periodic evaluations. Once a new best is found, it will appear here." } else { "Evolves as episodes complete. Once a new best is found, its network will appear here." };
                        draw_text_clamped(msg, panel.x, panel.y + panel.h * 0.5, 16.0, GRAY, panel.w - 8.0);
                    }
                }
            } else {
                let title = if state.last_best.is_finite() && state.last_best > f32::NEG_INFINITY {
                    if ECO_CONTINUOUS { format!("Best network (last eval, fit {:.2})", state.last_best) } else { format!("Best network (last gen {}, fit {:.2})", state.last_best_generation, state.last_best) }
                } else { "Best network (pending)".to_string() };
                draw_text_clamped(&title, panel.x, panel.y - 8.0, 18.0, LIGHTGRAY, panel.w - 8.0);
                if let Some(genome) = state.last_best_genome.as_ref() {
                    draw_network_panel(panel, genome);
                } else {
                    let msg = if ECO_CONTINUOUS { "Evolves with periodic evaluations. Once a new best is found, it will appear here." } else { "Evolves as episodes complete. Once a new best is found, its network will appear here." };
                    draw_text_clamped(msg, panel.x, panel.y + panel.h * 0.5, 16.0, GRAY, panel.w - 8.0);
                }
            }
        }
    }

    // HUD toast (bottom-right)
    if let Some((ref msg, ref mut secs)) = state.hud_toast {
        let dt = get_frame_time();
        let w = screen_width();
        let h = screen_height();
        *secs -= dt;
        let alpha = secs.clamp(0.0, 2.0) / 2.0; // 0..1
        let bg = Color::new(0.0, 0.0, 0.0, 0.7 * alpha);
        let fs = 18.0;
        let pad = 10.0;
        let tw = measure_text(msg, None, fs as u16, 1.0).width + 2.0 * pad;
        let th = fs + 2.0 * pad * 0.8;
        let tx = w - tw - 18.0;
        let ty = h - th - 18.0;
        draw_rectangle(tx, ty, tw, th, bg);
        draw_rectangle_lines(tx, ty, tw, th, 1.0, WHITE);
        draw_text(msg, tx + pad, ty + fs + (th - fs) * 0.5 - 6.0, fs, WHITE);
        if *secs <= 0.0 { state.hud_toast = None; }
    }

    // If paused, draw a centered semi-transparent overlay with "Paused"
    if !*running {
        let w = screen_width();
        let h = screen_height();
        // dim the world a bit
        draw_rectangle(0.0, 0.0, w, h, Color::new(0.0, 0.0, 0.0, 0.42));
        // big centered text
        let fs = 64.0;
        let label = "Paused";
        let dims = measure_text(label, None, fs as u16, 1.0);
        let tx = (w - dims.width) * 0.5;
        // y coordinate for draw_text is baseline, so center vertically roughly by adding half font size
        let ty = (h * 0.5) + (fs * 0.5);
        draw_text(label, tx, ty, fs, WHITE);
    }
}
