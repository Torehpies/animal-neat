use macroquad::prelude::*;
use neat::{
    config::EvolutionConfig,
    evolution,
    fitness::fitness_xor,
    genome::Genome,
    innovation_tracker::InnovationTracker,
    speciator::Speciator,
};

const GRID: usize = 60;

struct AppState {
    population: Vec<Genome>,
    innov: InnovationTracker,
    speciator: Speciator,
    generation: usize,
    target_fitness: f32,
    last_best: f32,
    last_avg: f32,
    cfg: EvolutionConfig,
}

impl AppState {
    fn new(pop_size: usize, num_inputs: u32, num_outputs: u32, target_fitness: f32) -> Self {
        let mut innov = InnovationTracker::new();
        let speciator = Speciator::new(2.0);
        let population = Genome::create_initial_population(pop_size, num_inputs, num_outputs, &mut innov);

        let cfg = EvolutionConfig { compatibility_threshold: 2.0, ..Default::default() };

        Self {
            population,
            innov,
            speciator,
            generation: 0,
            target_fitness,
            last_best: f32::NEG_INFINITY,
            last_avg: 0.0,
            cfg,
        }
    }

    fn best_genome(&self) -> &Genome {
        let best_idx = self.population
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.fitness.total_cmp(&b.fitness))
            .map(|(i, _)| i)
            .unwrap_or(0);
        &self.population[best_idx]
    }

    fn step_generation(&mut self) {
        // Score fitness
        let fitness_scores: Vec<f32> = self.population.iter().map(fitness_xor).collect();

        // Update tracking
        self.last_best = fitness_scores.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        self.last_avg = fitness_scores.iter().sum::<f32>() / fitness_scores.len() as f32;

        // Evolve
        self.population = evolution::evolution(
            std::mem::take(&mut self.population),
            fitness_scores,
            &mut self.speciator,
            &mut self.innov,
            &self.cfg,
        );
        self.generation += 1;
    }

    fn reset(&mut self, pop_size: usize, num_inputs: u32, num_outputs: u32) {
        self.generation = 0;
        self.last_best = f32::NEG_INFINITY;
        self.last_avg = 0.0;
        self.speciator = Speciator::new(self.cfg.compatibility_threshold);
        self.population = Genome::create_initial_population(pop_size, num_inputs, num_outputs, &mut self.innov);
    }
}

fn draw_heatmap(best: &Genome, area: Rect) {
    let cell_w = area.w / GRID as f32;
    let cell_h = area.h / GRID as f32;

    for gx in 0..GRID {
        for gy in 0..GRID {
            let x = gx as f32 / (GRID - 1) as f32;
            let y = gy as f32 / (GRID - 1) as f32;

            let out = best.evaluate_slice(&[x, y]);
            let v = out.get(0).copied().unwrap_or(0.0).clamp(0.0, 1.0);

            // Color: blend from blue (0) to orange (1)
            let color = Color::new(v, 0.3, 1.0 - v, 1.0);
            let px = area.x + gx as f32 * cell_w;
            // Flip Y for screen coordinates (0 at top)
            let py = area.y + (GRID - 1 - gy) as f32 * cell_h;
            draw_rectangle(px, py, cell_w + 1.0, cell_h + 1.0, color);
        }
    }
}

fn draw_points(area: Rect) {
    // XOR dataset
    let points = [
        ([0.0, 0.0], 0.0),
        ([0.0, 1.0], 1.0),
        ([1.0, 0.0], 1.0),
        ([1.0, 1.0], 0.0),
    ];

    let r = (area.w.min(area.h) / GRID as f32) * 1.5;

    for ([x, y], target) in points {
        let px = area.x + x * area.w;
        let py = area.y + (1.0 - y) * area.h;

        let color = if target > 0.5 {
            ORANGE
        } else {
            BLUE
        };
        draw_circle(px, py, r, color);
        draw_circle_lines(px, py, r, 2.0, BLACK);
    }
}

fn draw_hud(state: &AppState, area: Rect) {
    let margin = 16.0;
    let mut y = area.y + area.h + margin;

    let lines = [
        format!("Gen: {}", state.generation),
        format!("Best: {:.4}", state.last_best),
        format!("Avg: {:.4}", state.last_avg),
        format!("Target: {:.2}", state.target_fitness),
        "Controls: [Space]=step, [A]=auto-run toggle, [R]=reset".to_string(),
    ];

    for line in lines {
        draw_text(&line, area.x, y, 24.0, WHITE);
        y += 22.0;
    }
}

#[macroquad::main("NEAT XOR Visualizer")]
async fn main() {
    let mut state = AppState::new(300, 2, 1, 3.9);
    let mut autorun = false;
    let mut timer = 0.0;
    let step_interval = 0.002; // seconds per step when autorun is enabled
    
    loop {
        // Layout
        let w = screen_width();
        let h = screen_height();
        let size = w.min(h) * 0.8;
        let area = Rect { x: (w - size) * 0.5, y: (h - size) * 0.1, w: size, h: size };

        clear_background(DARKGRAY);

        // Controls
        if is_key_pressed(KeyCode::Space) {
            state.step_generation();
        }
        if is_key_pressed(KeyCode::A) {
            autorun = !autorun;
        }
        if is_key_pressed(KeyCode::R) {
            state.reset(300, 2, 1);
        }

        if autorun {
            timer += get_frame_time();
            if timer >= step_interval {
                state.step_generation();
                timer = 0.0;
            }
        }

        // Draw
        let best = state.best_genome();
        draw_heatmap(best, area);
        draw_points(area);
        draw_hud(&state, area);

        // Stop autorun if target reached (will still render)
        if state.last_best >= state.target_fitness {
            autorun = false;
            draw_text("Solved!", area.x, area.y - 10.0, 28.0, GREEN);
        }

        next_frame().await
    }
}