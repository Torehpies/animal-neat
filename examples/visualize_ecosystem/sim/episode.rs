
use macroquad::prelude::Vec2;
use ::rand::Rng;
use std::collections::VecDeque;
use neat::genome::Genome;
use crate::{params::*, sim::{Agent, AgentKind, CommSignal, StepDelta, tick_step, AgentId}, body::Body, world};
use rand::seq::SliceRandom;

pub struct Episode {
    pub food: Vec<Vec2>,
    pub food_lifetime: Vec<usize>,
    pub agents: Vec<Agent>,
    pub steps: usize,
    pub first_eat_step: Option<usize>,
    pub births_this_episode: usize,
    pub births_herb: usize,
    pub births_carn: usize,
    // Motor usage stats (for HUD): aggregated over agent-steps this episode
    pub total_agent_steps: usize,
    pub avg_speed_accum: f32,
    pub heading_change_accum: f32,
    pub comm_signals: Vec<CommSignal>,       // active broadcast resource signals
    pub comm_fitness_accum: Vec<f32>,        // per-agent communication reward accumulation
}

impl Episode {
    pub fn new<R: Rng>(rng: &mut R, herb_count: usize, carn_count: usize) -> Self {
        let agent_count = herb_count.saturating_add(carn_count);
        let mut agents = Vec::with_capacity(agent_count);

        // First create the requested number of herbivores, then carnivores.
        for i in 0..herb_count {
            let pos = world::rand_pos(rng);
            agents.push(Agent {
                id: AgentId(i),
                kind: AgentKind::Herbivore,
                body: Body { pos, vel: Vec2::new(0.0, 0.0), radius: AGENT_COLLISION_RADIUS },
                theta: -std::f32::consts::FRAC_PI_2,
                energy: crate::params::get_initial_energy_for(crate::params::Kind::Herb).min(crate::params::get_max_energy_for(crate::params::Kind::Herb)),
                health: AGENT_BASE_HEALTH,
                max_health: AGENT_BASE_HEALTH,
                invuln_steps: 0,
                alive_steps: 0,
                eaten: 0,
                consumed: false,
                kills: 0,
                predation_flash_steps: 0,
                dead_since: None,
                corpse_energy: 0.0,
                digest: VecDeque::new(),
                last_food_mem: Vec2 { x: 0.0, y: 0.0 },
                last_danger_mem: Vec2 { x: 0.0, y: 0.0 },
                last_same_mem: Vec2 { x: 0.0, y: 0.0 },
                last_other_mem: Vec2 { x: 0.0, y: 0.0 },
                // Two fixed ecological species: 0 = Herbivore, 1 = Carnivore
                species_id: 0,
                age_steps: 0,
                call_intensity: 0.0, heard_sectors: [0.0;3],
                repro_cooldown: 0,
                offspring_count: 0,
                attack_hits: 0,
                kills_caused: 0,
                idle_anchor: world::rand_pos(rng),
                idle_steps: 0,
                total_idle_steps: 0,
                total_idle_penalty: 0.0,
                input_buf: vec![0.0; crate::params::INPUTS],
                energy_accum: 0.0,
                herding_units: 0.0,
                approach_food_units: 0.0,
                chase_other_units: 0.0,
                chase_same_units: 0.0,
                flee_other_units: 0.0,
                hunger: 0.0,
                contentment: 1.0,
                rest_content_units: 0.0,
                eat_early_units: 0.0,
            });
        }
        for j in 0..carn_count {
            let i = herb_count + j;
            let pos = world::rand_pos(rng);
            agents.push(Agent {
                id: AgentId(i),
                kind: AgentKind::Carnivore,
                body: Body { pos, vel: Vec2::new(0.0, 0.0), radius: AGENT_COLLISION_RADIUS },
                theta: -std::f32::consts::FRAC_PI_2,
                energy: crate::params::get_initial_energy_for(crate::params::Kind::Carn).min(crate::params::get_max_energy_for(crate::params::Kind::Carn)),
                health: AGENT_BASE_HEALTH,
                max_health: AGENT_BASE_HEALTH,
                invuln_steps: 0,
                alive_steps: 0,
                eaten: 0,
                consumed: false,
                kills: 0,
                predation_flash_steps: 0,
                dead_since: None,
                corpse_energy: 0.0,
                digest: VecDeque::new(),
                last_food_mem: Vec2 { x: 0.0, y: 0.0 },
                last_danger_mem: Vec2 { x: 0.0, y: 0.0 },
                last_same_mem: Vec2 { x: 0.0, y: 0.0 },
                last_other_mem: Vec2 { x: 0.0, y: 0.0 },
                species_id: 1,
                age_steps: 0,
                call_intensity: 0.0, heard_sectors: [0.0;3],
                repro_cooldown: 0,
                offspring_count: 0,
                attack_hits: 0,
                kills_caused: 0,
                idle_anchor: world::rand_pos(rng),
                idle_steps: 0,
                total_idle_steps: 0,
                total_idle_penalty: 0.0,
                input_buf: vec![0.0; crate::params::INPUTS],
                energy_accum: 0.0,
                herding_units: 0.0,
                approach_food_units: 0.0,
                chase_other_units: 0.0,
                chase_same_units: 0.0,
                flee_other_units: 0.0,
                hunger: 0.0,
                contentment: 1.0,
                rest_content_units: 0.0,
                eat_early_units: 0.0,
            });
        }

        // Shuffle spawn order so indices aren't grouped by kind (keeps UI/speciation coloring stable)
        if agent_count > 1 {
            agents.shuffle(rng);
            // reassign stable AgentId indices after shuffle
            for (idx, a) in agents.iter_mut().enumerate() {
                a.id = AgentId(idx);
            }
        }
        let food = world::build_world(rng);
        let food_lifetime = world::init_food_lifetimes(&food, rng);
        Self {
            food,
            food_lifetime,
            agents,
            steps: 0,
            first_eat_step: None,
            births_this_episode: 0,
            births_herb: 0,
            births_carn: 0,
            total_agent_steps: 0,
            avg_speed_accum: 0.0,
            heading_change_accum: 0.0,
            comm_signals: Vec::new(),
            comm_fitness_accum: vec![0.0; agent_count],
        }
    }
    pub fn step<R: Rng>(&mut self, population: &[Genome], rng: &mut R) -> bool {
        if self.agents.iter().all(|a| a.energy <= 0.0 || a.health <= DEATH_HEALTH_THRESHOLD) { return false; }
        // Tick once using shared engine
        let stats: StepDelta = tick_step(
            population,
            &mut self.food,
            &mut self.food_lifetime,
            &mut self.agents,
            None,
            &mut self.comm_signals,
            &mut self.comm_fitness_accum,
            self.steps,
            rng,
            Some(&mut self.first_eat_step),
        );
        self.total_agent_steps += stats.total_agent_steps;
        self.avg_speed_accum += stats.avg_speed_accum;
        self.heading_change_accum += stats.heading_change_accum;
        self.steps += 1;
        true
    }

    pub fn is_finished(&self) -> bool {
        self.steps >= MAX_STEPS ||
        self.agents.iter().all(|a| a.energy <= 0.0 || a.health <= DEATH_HEALTH_THRESHOLD)
    }
}
