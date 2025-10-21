use macroquad::prelude::Texture2D;

use crate::sim::Episode;
use crate::ui_graphs::Trends;
use crate::ui_menu::SimConfig;
use crate::scoreboard;
use crate::params::*;

use neat::neat::{
    config::EvolutionConfig,
    evolution,
    genome::Genome,
    innovation_tracker::InnovationTracker,
    speciator::Speciator,
};

/// Application state: population, sim state and UI flags.
pub struct AppState {
    pub population: Vec<Genome>,
    pub innov: InnovationTracker,
    pub speciator: Speciator,
    pub cfg: EvolutionConfig,
    pub generation: usize,
    pub last_best: f32,
    pub last_avg: f32,
    pub episode: Episode,
    pub show_cones: bool,
    pub member_species: Vec<usize>,
    // Best of last evaluated generation (for HUD panel)
    pub last_best_generation: usize,
    pub last_best_genome: Option<Genome>,
    // Debug overlays
    pub show_unified_overlay: bool,
    pub show_energy_overlay: bool,
    pub show_collision_radii: bool,
    pub show_grid: bool,
    // HUD/network & focus controls
    pub show_best_network_panel: bool,
    pub focused_agent: Option<usize>,
    // Eco mode helpers
    pub eco_episode_counter: usize,
    pub show_controls: bool,
    pub color_by_species: bool,
    // Graphs
    pub show_graphs_panel: bool,
    pub graphs: Trends,
    // Separate overlay for graphs (full-screen modal style)
    pub show_graphs_overlay: bool,
    // Active tab for graphs overlay
    pub graphs_tab: crate::ui_graphs::GraphTab,
    // Runtime config
    #[allow(dead_code)]
    pub sim_config: SimConfig,
    // Per-session save prefix (unique per new simulation)
    pub save_prefix: String,
    // Sprites (optional). If not present, fallback shapes are used.
    pub herb_tex: Option<Texture2D>,
    pub carn_tex: Option<Texture2D>,
    pub plant_tex: Option<Texture2D>,
    pub meat_tex: Option<Texture2D>,
    // Diagnostics
    pub ultra_mode: bool,
    // Scoreboard modal
    pub show_scoreboard_panel: bool,
    pub scoreboard_pending: bool,
    pub scoreboard_rows: Vec<scoreboard::ScoreEntry>,
    // HUD quick-save feedback
    pub hud_toast: Option<(String, f32)>, // (message, remaining_secs)
    // Compact modal for less-important view toggles
    pub show_view_options_overlay: bool,
}

impl AppState {
    pub fn new(sim_config: SimConfig) -> Self {
        let pop_size = sim_config.population_size;
        let num_inputs = INPUTS as u32;
        let num_outputs = OUTPUTS as u32;
        let mut rng = ::rand::rng();
        let mut innov = InnovationTracker::new();
        // Speciation target and adapt rate are now configurable via params
        let mut speciator = Speciator::new(1.0).with_target(SPECIES_TARGET, SPECIES_ADAPT_RATE);
        let cfg = EvolutionConfig { compatibility_threshold: 2.0, ..Default::default() };
        let population = Genome::create_initial_population(pop_size, num_inputs, num_outputs, &mut innov);
        // initial speciation for coloring
        speciator.speciate(&population);
        let member_species = {
            let mut map = vec![0usize; population.len()];
            for (sidx, s) in speciator.get_species().iter().enumerate() {
                for &m in &s.members { if m < population.len() { map[m] = sidx; } }
            }
            map
        };
        // Ensure runtime world and energy/population settings reflect the provided SimConfig
        crate::world::set_runtime_config(sim_config.world_width, sim_config.world_height, sim_config.max_food, sim_config.food_respawn_prob);
        crate::params::set_runtime_energy_config_per_kind(
            sim_config.initial_energy_herb,
            sim_config.max_energy_herb,
            sim_config.energy_drain_per_step_herb,
            sim_config.initial_energy_carn,
            sim_config.max_energy_carn,
            sim_config.energy_drain_per_step_carn,
        );
        crate::params::set_runtime_population_size(sim_config.population_size);
        let episode = Episode::new(&mut rng, pop_size);
        // Create a unique save prefix per simulation using system time
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
        let save_prefix = format!("snapshots/sim_{}_{:09}", now.as_secs(), now.subsec_nanos());
        Self {
            population,
            innov,
            speciator,
            cfg,
            generation: 0,
            last_best: f32::NEG_INFINITY,
            last_avg: 0.0,
            episode,
            show_cones: false,
            member_species,
            last_best_generation: 0,
            last_best_genome: None,
            show_unified_overlay: false,
            show_energy_overlay: false,
            show_collision_radii: false,
            show_grid: false,
            show_best_network_panel: false,
            focused_agent: None,
            eco_episode_counter: 0,
            show_controls: true,
            color_by_species: true,
            show_graphs_panel: false,
            graphs: Trends::new(),
            show_graphs_overlay: false,
            graphs_tab: crate::ui_graphs::GraphTab::Population,
            sim_config,
            save_prefix,
            herb_tex: None,
            carn_tex: None,
            plant_tex: None,
            meat_tex: None,
            ultra_mode: false,
            show_scoreboard_panel: false,
            scoreboard_pending: false,
            scoreboard_rows: Vec::new(),
            hud_toast: None,
            show_view_options_overlay: false,
        }
    }

    pub fn eval_population(&mut self) -> Vec<f32> {
        // Multi-episode averaging with exploration reward and avoidance penalty
        let mut acc = vec![0.0f32; self.population.len()];
        for _ in 0..EPISODES_PER_GEN {
            let scores = crate::sim::eval_population_single_episode(&self.population);
            for (i, s) in scores.iter().enumerate() { acc[i] += *s; }
        }
        for v in &mut acc { *v /= EPISODES_PER_GEN as f32; }
        let best = acc.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        let avg = acc.iter().sum::<f32>() / acc.len() as f32;
        self.last_best = best;
        self.last_avg = avg;
        acc
    }

    pub fn evolve_one_generation(&mut self) {
        let fitness_scores = self.eval_population();
        // Update best-ever before population is replaced
        if let Some((best_idx, _)) = fitness_scores
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
        {
            // Track best of last evaluated generation for HUD graph
            self.last_best_generation = self.generation;
            if best_idx < self.population.len() {
                self.last_best_genome = Some(self.population[best_idx].clone());
            } else {
                self.last_best_genome = None;
            }
        }
        // Evolve population
        self.population = evolution::evolution(
            std::mem::take(&mut self.population),
            fitness_scores,
            &mut self.speciator,
            &mut self.innov,
            &self.cfg,
        );
        // IMPORTANT: speciate BEFORE creating the next episode so agents get correct species IDs
        self.speciator.speciate(&self.population);
        self.member_species = {
            let mut map = vec![0usize; self.population.len()];
            for (sidx, s) in self.speciator.get_species().iter().enumerate() {
                for &m in &s.members { if m < self.population.len() { map[m] = sidx; } }
            }
            map
        };
        self.generation += 1;
        // Re-apply runtime config in case sim_config changed before creating the episode
        let sc = &self.sim_config;
        crate::world::set_runtime_config(sc.world_width, sc.world_height, sc.max_food, sc.food_respawn_prob);
        crate::params::set_runtime_energy_config_per_kind(
            sc.initial_energy_herb,
            sc.max_energy_herb,
            sc.energy_drain_per_step_herb,
            sc.initial_energy_carn,
            sc.max_energy_carn,
            sc.energy_drain_per_step_carn,
        );
        crate::params::set_runtime_population_size(sc.population_size);
        let mut rng = ::rand::rng();
        self.episode = Episode::new(&mut rng, self.population.len());
        // Clear focused agent because indices now refer to new episode
        self.focused_agent = None;
        // Optional periodic snapshotting after evolution completes this generation
        if SNAPSHOT_INTERVAL > 0 && self.generation % SNAPSHOT_INTERVAL == 0 {
            let filename = format!("{}__gen{:0>6}.json", self.save_prefix, self.generation);
            if let Err(e) = crate::snapshot::save_sim_snapshot(&filename, self.generation, &self.population, &self.innov, &self.episode, &self.member_species) {
                eprintln!("Auto-snapshot failed: {e}");
            } else {
                println!("Auto-saved sim snapshot to {filename}");
            }
        }
    }
}
