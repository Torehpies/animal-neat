use macroquad::prelude::*;
use crate::AppState;
use crate::params::*;
use crate::ui_common::{species_color, draw_text_clamped};
use crate::ui_network::draw_network_panel;

pub fn draw_hud(area: Rect, state: &AppState, running: bool, fast_mode: bool, member_species: &[usize]) {
    // Sidebar panel to avoid overflow
    let padding = 12.0;
    let mut y = area.y + padding;
    let x = area.x + padding;
    // Reserve bottom portion for network panel
    let network_h = (area.h * 0.42).clamp(160.0, 380.0);
    let max_y = area.y + area.h - padding - network_h - 8.0;
    let font_size = 18.0;
    let total_eaten: usize = state.episode.agents.iter().map(|a| a.eaten).sum();
    let alive = state.episode.agents.iter().filter(|a| a.energy > 0.0).count();
    let (min_energy, max_energy, avg_energy) = if !state.episode.agents.is_empty() {
        let mut min_e = f32::INFINITY; let mut max_e = f32::NEG_INFINITY; let mut sum = 0.0;
        for a in &state.episode.agents { min_e = min_e.min(a.energy); max_e = max_e.max(a.energy); sum += a.energy; }
        (min_e, max_e, sum / state.episode.agents.len() as f32)
    } else { (0.0, 0.0, 0.0) };
    let corpses = state.episode.agents.iter().filter(|a| a.energy <= 0.0 && !a.consumed).count();
    let plants = state.episode.food.len();
    let species_count = state.speciator.get_species().len();
    let first_eat = state.episode.first_eat_step.map(|s| s.to_string()).unwrap_or("-".to_string());
    let mode = if !running { "Paused" } else if fast_mode { "Running (Fast)" } else { "Running (Normal)" };
    let lines = vec![
        format!("Generation: {}", state.generation),
        format!("Population: {}  Species: {}", state.population.len(), species_count),
        format!("Mode: {}", mode),
        format!("Best: {:.3}  Avg: {:.3}", state.last_best, state.last_avg),
        format!("Steps: {}  First eat: {}", state.episode.steps, first_eat),
        format!("Plants: {}  Corpses: {}", plants, corpses),
        format!("Alive: {}  Total eaten: {}", alive, total_eaten),
        format!("Energy min/avg/max: {:.0} / {:.0} / {:.0}", min_energy, avg_energy, max_energy),
        format!("Inputs: {}  Rays: {}  Range: {:.0}", INPUTS, VISION_RAYS, VISION_RANGE),
    format!("Move: turn={:.2} rad  speed={:.1}", MAX_TURN, MAX_SPEED),
    format!("Turn cost: {:.3}  Thrust coupling: {:.2}", TURN_COST, THRUST_TURN_COUPLING),
    format!("Sprint x{:.2} (+{:.2})  Brake x{:.2} (+{:.2})  noise±{:.2}", SPRINT_MULT, SPRINT_COST, BRAKE_MULT, BRAKE_COST, MOTOR_NOISE),
        format!("Food vec range: {:.0}", FOOD_VECTOR_MAX_RANGE),
        format!("Danger vec range: {:.0}", DANGER_VECTOR_MAX_RANGE),
        format!("Density: sectors={} radius={:.0}", DENSITY_SECTORS, DENSITY_RADIUS),
        format!("Corpse decay: {:.1}%/step  Digest (plant/meat): {}/{} steps", CORPSE_DECAY_RATE*100.0, DIGEST_STEPS_PLANT, DIGEST_STEPS_MEAT),
        format!("Eaten weight: {:.2}  Step weight: {:.3}  Expl/cell: {:.3}", EAT_WEIGHT, STEP_WEIGHT, EXPL_REWARD_PER_CELL),
        format!("Predation: {}  Scavenge: {}  Meat energy: {:.0}", PREDATION_ENABLED, SCAVENGE_ENABLED, MEAT_ENERGY),
        "Controls:".to_string(),
        "  [P] pause/resume   [F] fast/normal".to_string(),
        "  [R] reset episode  [V] toggle vision  [D] density overlay  [B] vector overlay".to_string(),
        "  Hover an agent in the world to see overlays".to_string(),
        "Species (last gen):".to_string(),
    ];
    let mut species = state.last_species.clone();
    species.sort_by(|a, b| b.best_fitness.partial_cmp(&a.best_fitness).unwrap_or(std::cmp::Ordering::Equal));
    // Panel background
    draw_rectangle(area.x, area.y, area.w, area.h, Color::new(0.08, 0.08, 0.10, 0.95));
    draw_rectangle_lines(area.x, area.y, area.w, area.h, 2.0, GRAY);
    for line in lines {
        if y > max_y { break; }
        draw_text_clamped(&line, x, y, font_size, WHITE, area.w - (x - area.x) - padding);
    }
    // Species lines with color swatch
    for (rank, s) in species.into_iter().enumerate() {
        if y > max_y { break; }
        let color = species_color(rank);
        // swatch
        let sw_h = font_size * 0.8;
        let sw_w = sw_h * 1.4;
        draw_rectangle(x, y - sw_h + 2.0, sw_w, sw_h, color);
        draw_rectangle_lines(x, y - sw_h + 2.0, sw_w, sw_h, 1.0, BLACK);
        let text = format!(
            "  mem={} best={:.2} adj={:.2} stagn={} rep={} (rank #{:02})",
            s.members.len(), s.best_fitness, s.adjusted_fitness, s.stagnant_generations, s.representative, rank
        );
        draw_text(&text, x + sw_w + 6.0, y, font_size, WHITE);
        y += font_size + 6.0;
    }
    if y <= max_y {
        // Live predation summary
        let mut max_idx = 0usize;
        for &sidx in member_species { if sidx > max_idx { max_idx = sidx; } }
        let mut kills_per_species = vec![0usize; max_idx + 1];
        let mut preds_per_species = vec![0usize; max_idx + 1];
        for a in &state.episode.agents {
            if a.kills > 0 && a.id.0 < member_species.len() {
                let sidx = member_species[a.id.0];
                kills_per_species[sidx] += a.kills;
                preds_per_species[sidx] += 1;
            }
        }
        // Header
        draw_text("Predation (live):", x, y, font_size, WHITE);
        y += font_size + 6.0;
        for sidx in 0..kills_per_species.len() {
            if kills_per_species[sidx] == 0 { continue; }
            let color = species_color(sidx);
            let sw_h = font_size * 0.8; let sw_w = sw_h * 1.4;
            draw_rectangle(x, y - sw_h + 2.0, sw_w, sw_h, color);
            draw_rectangle_lines(x, y - sw_h + 2.0, sw_w, sw_h, 1.0, BLACK);
            let text = format!("  kills={} preds={}", kills_per_species[sidx], preds_per_species[sidx]);
            draw_text_clamped(&text, x + sw_w + 6.0, y, font_size, WHITE, area.w - (x + sw_w + 6.0 - area.x) - padding);
            y += font_size + 6.0;
            if y > max_y { break; }
        }
    }
    // Motor usage (live)
    if y <= max_y {
        let steps = state.episode.total_agent_steps.max(1) as f32;
        let avg_thrust = state.episode.thrust_sum / steps;
        let avg_turn = state.episode.abs_turn_sum / steps;
        let pct_sprint = (state.episode.sprint_used as f32) / steps * 100.0;
        let pct_brake = (state.episode.brake_used as f32) / steps * 100.0;
        let line = format!("Motor: thrust_avg={:.2}  |turn|_avg={:.2}  sprint={:.0}%  brake={:.0}%", avg_thrust, avg_turn, pct_sprint, pct_brake);
        draw_text_clamped(&line, x, y, font_size, WHITE, area.w - (x - area.x) - padding);
    }

    // Draw best-network panel at bottom
    let panel = Rect {
        x: area.x + 8.0,
        y: area.y + area.h - network_h + 8.0,
        w: area.w - 16.0,
        h: network_h - 16.0,
    };
    draw_rectangle(panel.x - 4.0, panel.y - 4.0, panel.w + 8.0, panel.h + 8.0, Color::new(0.05, 0.05, 0.07, 0.95));
    draw_rectangle_lines(panel.x - 4.0, panel.y - 4.0, panel.w + 8.0, panel.h + 8.0, 2.0, Color::new(0.25, 0.25, 0.3, 1.0));
    let title = if state.last_best.is_finite() && state.last_best > f32::NEG_INFINITY {
        format!("Best network (last gen {}, fit {:.2})", state.last_best_generation, state.last_best)
    } else {
        "Best network (pending)".to_string()
    };
    draw_text_clamped(&title, panel.x, panel.y - 8.0, 18.0, LIGHTGRAY, panel.w - 8.0);
    if let Some(genome) = state.last_best_genome.as_ref() {
        draw_network_panel(panel, genome);
    } else {
        let msg = "Evolves as episodes complete. Once a new best is found, its network will appear here.";
        draw_text_clamped(msg, panel.x, panel.y + panel.h * 0.5, 16.0, GRAY, panel.w - 8.0);
    }
}
