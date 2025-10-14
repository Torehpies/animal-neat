use macroquad::prelude::*;
use crate::AppState;
use crate::params::*;
use crate::ui_common::{draw_text_clamped, draw_text_wrapped};
use crate::ui_network::{draw_network_panel, draw_network_panel_activations};

// Compact HUD with essential stats; best-network panel retained.
pub fn draw_hud(area: Rect, state: &AppState, running: bool, fast_mode: bool, _member_species: &[usize]) {
    // Panel background and border
    draw_rectangle(area.x, area.y, area.w, area.h, Color::new(0.08, 0.08, 0.10, 0.95));
    draw_rectangle_lines(area.x, area.y, area.w, area.h, 2.0, GRAY);

    let padding = 12.0;
    let font = 18.0;
    let line_h = font + 6.0;
    let x = area.x + padding;
    let mut y = area.y + padding;

    // Reserve bottom portion for network panel; can be hidden via toggles
    let network_h = if state.show_best_network_panel || state.show_live_network { (area.h * 0.42).clamp(160.0, 380.0) } else { 0.0 };
    let max_y = area.y + area.h - padding - network_h - 8.0;
    let max_w = area.w - (x - area.x) - padding;

    // Derived stats
    let mode = if !running { "Paused" } else if fast_mode { "Running (Fast)" } else { "Running (Normal)" };
    let species_count = state.speciator.get_species().len();
    let alive = state.episode.agents.iter().filter(|a| a.energy > 0.0).count();
    let plants = state.episode.food.len();
    let corpses = state.episode.agents.iter().filter(|a| a.energy <= 0.0 && !a.consumed).count();
    let (min_e, avg_e, max_e) = if !state.episode.agents.is_empty() {
        let mut min_e = f32::INFINITY; let mut max_e = f32::NEG_INFINITY; let mut sum = 0.0;
        for a in &state.episode.agents { min_e = min_e.min(a.energy); max_e = max_e.max(a.energy); sum += a.energy; }
        (min_e, sum / state.episode.agents.len() as f32, max_e)
    } else { (0.0, 0.0, 0.0) };
    // Satiety summary (0..1)
    let (min_s, avg_s, max_s) = if !state.episode.agents.is_empty() {
        let mut min_s = f32::INFINITY; let mut max_s = f32::NEG_INFINITY; let mut sum_s = 0.0;
        for a in &state.episode.agents { min_s = min_s.min(a.satiety); max_s = max_s.max(a.satiety); sum_s += a.satiety; }
        (min_s, sum_s / state.episode.agents.len() as f32, max_s)
    } else { (0.0, 0.0, 0.0) };

    // Essentials block
    let essentials = [
        if ECO_CONTINUOUS {
            format!("ECO mode • Ep {} — {}", state.eco_episode_counter, mode)
        } else {
            format!("Gen {} — {}", state.generation, mode)
        },
        format!("Pop {} • Species {}", state.population.len(), species_count),
        format!("Best {:.2} • Avg {:.2}", state.last_best, state.last_avg),
        format!("Steps {} • Alive {}", state.episode.steps, alive),
        if ECO_CONTINUOUS {
            format!("Births this ep: {}", state.episode.births_this_episode)
        } else {
            format!("Plants {} • Corpses {}", plants, corpses)
        },
        if ECO_CONTINUOUS {
            format!("Plants {} • Corpses {}", plants, corpses)
        } else {
            format!("Energy min/avg/max: {:.0}/{:.0}/{:.0}", min_e, avg_e, max_e)
        },
            format!("Satiety avg: {:.2} (min {:.2} max {:.2})", avg_s, min_s, max_s),
        format!("Inputs: {}", INPUTS),
    ];
    for line in essentials.iter() {
        if y > max_y { break; }
        y = draw_text_wrapped(line, x, y, font, WHITE, max_w, 6.0);
    }

    // Optional: Biome plant distribution summary
    if y <= max_y {
        use crate::params::{BIOME_X_SPLITS};
        let mut counts = [0usize; 3];
        for f in &state.episode.food {
            let nx = f.x / WORLD_W;
            let bi = if nx < BIOME_X_SPLITS[0] { 0 } else if nx < BIOME_X_SPLITS[1] { 1 } else { 2 };
            counts[bi] += 1;
        }
        let biome_line = format!("Plants per biome: [{} | {} | {}]", counts[0], counts[1], counts[2]);
        y = draw_text_wrapped(&biome_line, x, y, 16.0, GRAY, max_w, 6.0);
    }

    // Movement stats (relative turn + speed)
    if y <= max_y {
        let steps = state.episode.total_agent_steps.max(1) as f32;
        let avg_speed = state.episode.avg_speed_accum / steps;
        let avg_heading_change = state.episode.heading_change_accum / steps; // mean absolute turn delta
        let motor = format!("Move: v {:.2} • dθ {:.3}", avg_speed, avg_heading_change);
        draw_text_clamped(&motor, x, y, font, WHITE, max_w);
        y += line_h;
    }

    // Focused agent detail block (if any)
    if y <= max_y {
        if let Some(fi) = state.focused_agent {
            if let Some(a) = state.episode.agents.get(fi) {
                let hdr = format!("Focused Agent #{}", fi);
                y = draw_text_wrapped(&hdr, x, y, 18.0, LIGHTGRAY, max_w, 4.0) + 2.0;
                let alive = a.energy > 0.0 && a.health > 0.0;
                let diet_plants = a.eaten.saturating_sub(a.kills) as f32;
                let diet_meat = a.kills as f32;
                let total_intake = diet_plants + diet_meat;
                let meat_ratio = if total_intake > 0.0 { diet_meat / total_intake } else { 0.0 };
                let lines = [
                    format!("Species {} • Births {}", a.species_id, a.offspring_count),
                    format!("Status: {}", if alive { "Alive" } else { "Dead" }),
                    format!("Energy {:.0}/{:.0}", a.energy.max(0.0), MAX_ENERGY),
                    format!("Satiety {:.2}", a.satiety),
                    format!("Health {:.0}/{:.0}", a.health.max(0.0), a.max_health),
                    format!("Alive steps {}", a.alive_steps),
                    format!("Intake plants:{} meat:{} (meat% {:.0}%)", diet_plants as i32, diet_meat as i32, meat_ratio*100.0),
                    format!("Kills {} CorpseEnergy {:.0}", a.kills, a.corpse_energy),
                ];
                for line in lines.iter() { if y > max_y { break; } y = draw_text_wrapped(line, x+4.0, y, 16.0, GRAY, max_w, 4.0); }
            }
        }
    }

    // Controls (toggleable) — always allow when enabled, even in simple HUD
    if state.show_controls {
        if y <= max_y {
            let header = "Controls";
            y = draw_text_wrapped(header, x, y, 18.0, LIGHTGRAY, max_w, 6.0) + 2.0;
            let lines = [
                "[P] Pause/Resume   [F] Fast Mode   [R] Reset Episode   [Esc] Clear Focus",
                "[Click] Focus Agent   [N] Toggle Best Panel   [M] Toggle Live Net   [H] Toggle Controls   [K] Color: Species/Diet",
                "[V] Show/Hide Vision Rays   [U] Unified Overlay (vision grid + memory)",
                "[E] Energy Bar   [C] Collision Radii   [G] Exploration Grid",
                if ECO_CONTINUOUS { "[S] Save Population Snapshot   [B] Toggle Easy Births" } else { "[S] Save Population Snapshot" },
            ];
            for line in lines.iter() {
                if y > max_y { break; }
                y = draw_text_wrapped(line, x + 6.0, y, 16.0, GRAY, max_w, 6.0);
            }
        }
    } else {
        // Compact hint when controls are hidden
        if y <= max_y {
            let hint = "[H] Show Controls";
            let _ = draw_text_wrapped(hint, x, y, 16.0, GRAY, max_w, 6.0);
        }
    }

    // Network panel area (for best or live activations)
    if (state.show_best_network_panel || state.show_live_network) && network_h > 0.0 {
        let panel = Rect { x: area.x + 8.0, y: area.y + area.h - network_h + 8.0, w: area.w - 16.0, h: network_h - 16.0 };
        draw_rectangle(panel.x - 4.0, panel.y - 4.0, panel.w + 8.0, panel.h + 8.0, Color::new(0.05, 0.05, 0.07, 0.95));
        draw_rectangle_lines(panel.x - 4.0, panel.y - 4.0, panel.w + 8.0, panel.h + 8.0, 2.0, Color::new(0.25, 0.25, 0.3, 1.0));
        
        // Determine which genome to display
        let (genome_to_show, title) = if let Some(fi) = state.focused_agent {
            // Show focused agent's genome
            if fi < state.population.len() {
                (Some(&state.population[fi]), format!("Agent #{} Network", fi))
            } else {
                (None, "Invalid agent index".to_string())
            }
        } else {
            // Show best genome from last generation
            let title = if state.last_best.is_finite() && state.last_best > f32::NEG_INFINITY {
                format!("Best network (gen {}, fit {:.2})", state.last_best_generation, state.last_best)
            } else { "Best network (pending)".to_string() };
            (state.last_best_genome.as_ref(), title)
        };
        
        draw_text_clamped(&title, panel.x, panel.y - 8.0, 18.0, LIGHTGRAY, panel.w - 8.0);
        if let Some(genome) = genome_to_show {
            draw_network_panel(panel, genome);
        } else {
            let msg = "Evolves as episodes complete. Once a new best is found, its network will appear here.";
            draw_text_clamped(msg, panel.x, panel.y + panel.h * 0.5, 16.0, GRAY, panel.w - 8.0);
        }
    }
}
