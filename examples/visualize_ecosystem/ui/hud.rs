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

    // Controls (toggleable) rendered as clickable buttons instead of keybind labels
    if state.show_controls {
        if y <= max_y {
            y = section_title("Controls", x, y, max_w);
            let col_gap = 12.0;
            let col_w = (max_w - col_gap) * 0.5;
            let fs = 16.5;
            let btn_h = fs + 8.0;
            let btn_pad_x = 12.0;

            // Left column: Reset, Best Network, Toggle Controls
            let mut yl = y;
            let left = ["Reset", "Best Network", "Controls"];
            for (i, label) in left.iter().enumerate() {
                if yl > max_y { break; }
                let bx = x + 6.0;
                let by = yl - fs + 4.0;
                let bw = col_w - 12.0;
                let (mx, my) = mouse_position();
                let hovering = mx >= bx && mx <= bx + bw && my >= by && my <= by + btn_h;
                // Determine 'on' state for color
                let on = match i {
                    0 => false, // Reset is an action
                    1 => state.show_best_network_panel,
                    2 => state.show_controls,
                    _ => false,
                };
                let bg = if on { Color::new(0.22, 0.58, 0.95, 1.0) } else if hovering { Color::new(0.18, 0.18, 0.18, 1.0) } else { Color::new(0.12, 0.12, 0.12, 0.9) };
                draw_rectangle(bx, by, bw, btn_h, bg);
                draw_rectangle_lines(bx, by, bw, btn_h, 1.0, Color::new(0.6,0.6,0.6,0.8));
                // label centered vertically
                draw_text(label, bx + btn_pad_x, by + (btn_h * 0.65), fs, WHITE);
                if hovering && is_mouse_button_pressed(MouseButton::Left) {
                    match i {
                        0 => {
                            // Reset episode
                            let mut rng = ::rand::rng();
                            state.episode = crate::sim::Episode::new(&mut rng, state.population.len());
                            state.focused_agent = None;
                            state.hud_toast = Some(("Episode reset".to_string(), 1.6));
                        }
                        1 => { state.show_best_network_panel = !state.show_best_network_panel; }
                        2 => { state.show_controls = !state.show_controls; }
                        _ => {}
                    }
                }
                // description to the right of button
                let desc_x = bx + bw + 8.0;
                let desc_w = (x + col_w) - desc_x;
                let desc = match i {
                    0 => "Restart the episode (randomized spawns)",
                    1 => "Show best network in panel",
                    2 => "Hide/show these controls",
                    _ => "",
                };
                draw_text_clamped(desc, desc_x, yl, 14.0, LIGHTGRAY, desc_w);
                yl += btn_h + 8.0;
            }

            // Right column: compact primary controls + View Options button
            let mut yr = y;
            let right_x = x + 6.0 + col_w + col_gap;
            let right = [
                "View Options",
                "Graphs Panel",
                "Quick Save",
                "Load Latest",
            ];
            for (i, label) in right.iter().enumerate() {
                if yr > max_y { break; }
                let bx = right_x;
                let by = yr - fs + 4.0;
                let bw = col_w - 12.0;
                let (mx, my) = mouse_position();
                let hovering = mx >= bx && mx <= bx + bw && my >= by && my <= by + btn_h;
                let on = match i {
                    0 => false, // View Options (action)
                    1 => state.show_graphs_overlay,
                    2 => false,
                    3 => false,
                    _ => false,
                };
                let bg = if on { Color::new(0.22, 0.58, 0.95, 1.0) } else if hovering { Color::new(0.18, 0.18, 0.18, 1.0) } else { Color::new(0.12, 0.12, 0.12, 0.9) };
                draw_rectangle(bx, by, bw, btn_h, bg);
                draw_rectangle_lines(bx, by, bw, btn_h, 1.0, Color::new(0.6,0.6,0.6,0.8));
                draw_text(label, bx + btn_pad_x, by + (btn_h * 0.65), fs, WHITE);
                if hovering && is_mouse_button_pressed(MouseButton::Left) {
                    match i {
                        0 => { state.show_view_options_overlay = !state.show_view_options_overlay; }
                        1 => { state.show_graphs_overlay = !state.show_graphs_overlay; }
                        2 => { state.hud_toast = Some(("Quick save not implemented in HUD".to_string(), 2.0)); }
                        3 => { state.hud_toast = Some(("Load latest not implemented".to_string(), 2.0)); }
                        _ => {}
                    }
                }
                // optional small help text under the label
                let help = match i {
                    0 => "Open the View Options modal",
                    1 => "Open graphs panel overlay",
                    2 => "Save a quick snapshot of the sim",
                    3 => "Load the most recent quicksave",
                    _ => "",
                };
                draw_text_clamped(help, bx + bw + 8.0, yr, 13.0, GRAY, (x + max_w) - (bx + bw + 8.0));
                yr += btn_h + 8.0;
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

    // View Options overlay modal (contains the less-important toggles)
    if state.show_view_options_overlay {
        let w = screen_width();
        let h = screen_height();
        // dim background
        draw_rectangle(0.0, 0.0, w, h, Color::new(0.0, 0.0, 0.0, 0.5));

        let modal_w = 520.0f32;
        let modal_h = 320.0f32;
        let mx = (w - modal_w) * 0.5;
        let my = (h - modal_h) * 0.5;
        let frame = Rect { x: mx - 6.0, y: my - 6.0, w: modal_w + 12.0, h: modal_h + 12.0 };
        draw_panel(frame, SUBPANEL_BG, PANEL_BORDER, 2.0);

        // Title
        draw_text_clamped("View Options", mx + 12.0, my + 8.0, 22.0, LIGHTGRAY, modal_w - 24.0);
        // Description
        draw_text_clamped("Less-important display toggles (move to overlay for compact HUD)", mx + 12.0, my + 36.0, 14.0, GRAY, modal_w - 24.0);

        // Buttons (2 columns × 3 rows)
        let col_gap = 18.0;
        let col_w = (modal_w - 24.0 - col_gap) * 0.5;
    let btn_fs = 18.0;
    let btn_h = btn_fs + 12.0;
    let start_x = mx + 12.0;
    let by = my + 76.0;

    let opts: Vec<(&str, Box<dyn Fn(&AppState) -> bool>, Box<dyn Fn(&mut AppState)>)> = vec![
            ("Pause at End", Box::new(|s: &AppState| { s.show_scoreboard_panel }), Box::new(|s: &mut AppState| { s.show_scoreboard_panel = !s.show_scoreboard_panel })),
            ("Collision", Box::new(|s: &AppState| { s.show_collision_radii }), Box::new(|s: &mut AppState| { s.show_collision_radii = !s.show_collision_radii })),
            ("Vision Rays", Box::new(|s: &AppState| { s.show_cones }), Box::new(|s: &mut AppState| { s.show_cones = !s.show_cones })),
            ("Unified Overlay", Box::new(|s: &AppState| { s.show_unified_overlay }), Box::new(|s: &mut AppState| { s.show_unified_overlay = !s.show_unified_overlay })),
            ("Energy Bar", Box::new(|s: &AppState| { s.show_energy_overlay }), Box::new(|s: &mut AppState| { s.show_energy_overlay = !s.show_energy_overlay })),
            ("Exploration Grid", Box::new(|s: &AppState| { s.show_grid }), Box::new(|s: &mut AppState| { s.show_grid = !s.show_grid })),
        ];

        for col in 0..2 {
            let bx = start_x + col as f32 * (col_w + col_gap);
            let mut row_y = by;
            for row in 0..3 {
                let idx = col * 3 + row;
                if idx >= opts.len() { break; }
                let (label, getter, _) = &opts[idx];
                let on = getter(&*state);
                let (mx_mouse, my_mouse) = mouse_position();
                let hovering = mx_mouse >= bx && mx_mouse <= bx + col_w && my_mouse >= row_y && my_mouse <= row_y + btn_h;
                let bg = if on { Color::new(0.22, 0.58, 0.95, 1.0) } else if hovering { Color::new(0.18, 0.18, 0.18, 1.0) } else { Color::new(0.12, 0.12, 0.12, 0.9) };
                draw_rectangle(bx, row_y, col_w, btn_h, bg);
                draw_rectangle_lines(bx, row_y, col_w, btn_h, 1.0, Color::new(0.6,0.6,0.6,0.8));
                draw_text(label, bx + 12.0, row_y + (btn_h * 0.68), btn_fs, WHITE);
                if hovering && is_mouse_button_pressed(MouseButton::Left) {
                    let (_, _, setter) = &opts[idx];
                    setter(state);
                }
                row_y += btn_h + 12.0;
            }
        }

        // Close button
        let close_w = 120.0;
        let close_h = 36.0;
        let cx = mx + modal_w - close_w - 16.0;
        let cy = my + modal_h - close_h - 16.0;
        let (mx_mouse, my_mouse) = mouse_position();
        let hovering_close = mx_mouse >= cx && mx_mouse <= cx + close_w && my_mouse >= cy && my_mouse <= cy + close_h;
        let close_bg = if hovering_close { Color::new(0.18, 0.18, 0.18, 1.0) } else { Color::new(0.12, 0.12, 0.12, 0.9) };
        draw_rectangle(cx, cy, close_w, close_h, close_bg);
        draw_rectangle_lines(cx, cy, close_w, close_h, 1.0, Color::new(0.6,0.6,0.6,0.8));
        draw_text("Close", cx + 20.0, cy + (close_h * 0.68), 20.0, WHITE);
        if hovering_close && is_mouse_button_pressed(MouseButton::Left) {
            state.show_view_options_overlay = false;
        }
    }
}
