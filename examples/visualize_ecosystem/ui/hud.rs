use macroquad::prelude::*;
use crate::AppState;
use crate::params::*;
use crate::ui_common::{draw_text_clamped, draw_text_wrapped};
use crate::ui_network::draw_network_panel;

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

    // Reserve bottom portion for network panel (unchanged)
    let network_h = (area.h * 0.42).clamp(160.0, 380.0);
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

    // Essentials block
    let essentials = [
        format!("Gen {} — {}", state.generation, mode),
        format!("Pop {} • Species {}", state.population.len(), species_count),
        format!("Best {:.2} • Avg {:.2}", state.last_best, state.last_avg),
        format!("Steps {} • Alive {}", state.episode.steps, alive),
        format!("Plants {} • Corpses {}", plants, corpses),
        format!("Energy min/avg/max: {:.0}/{:.0}/{:.0}", min_e, avg_e, max_e),
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

    // Controls (compact)
    if y <= max_y {
    // Updated toggles: Vision Rays(V), Pooled Sectors(S), Density (D), Memory/Target Vectors (B)
    let controls = "Controls: [P] pause  [F] fast  [R] reset  [V] rays  [S] pooled  [D] density  [B] vectors";
        let _ = draw_text_wrapped(controls, x, y, 16.0, GRAY, max_w, 6.0);
        // no need to update y further; panel starts below
    }

    // Best-network panel (unchanged)
    let panel = Rect { x: area.x + 8.0, y: area.y + area.h - network_h + 8.0, w: area.w - 16.0, h: network_h - 16.0 };
    draw_rectangle(panel.x - 4.0, panel.y - 4.0, panel.w + 8.0, panel.h + 8.0, Color::new(0.05, 0.05, 0.07, 0.95));
    draw_rectangle_lines(panel.x - 4.0, panel.y - 4.0, panel.w + 8.0, panel.h + 8.0, 2.0, Color::new(0.25, 0.25, 0.3, 1.0));
    let title = if state.last_best.is_finite() && state.last_best > f32::NEG_INFINITY {
        format!("Best network (last gen {}, fit {:.2})", state.last_best_generation, state.last_best)
    } else { "Best network (pending)".to_string() };
    draw_text_clamped(&title, panel.x, panel.y - 8.0, 18.0, LIGHTGRAY, panel.w - 8.0);
    if let Some(genome) = state.last_best_genome.as_ref() {
        draw_network_panel(panel, genome);
    } else {
        let msg = "Evolves as episodes complete. Once a new best is found, its network will appear here.";
        draw_text_clamped(msg, panel.x, panel.y + panel.h * 0.5, 16.0, GRAY, panel.w - 8.0);
    }
}
