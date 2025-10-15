
use macroquad::prelude::Vec2;
use ::rand::Rng;
use std::collections::VecDeque;
use neat::genome::Genome;
use crate::{params::*, sim::{Agent, CommSignal, StepDelta, tick_step, AgentId}, body::Body, world};

pub struct Episode {
    pub food: Vec<Vec2>,
    pub food_lifetime: Vec<usize>,
    pub agents: Vec<Agent>,
    pub steps: usize,
    pub first_eat_step: Option<usize>,
    pub births_this_episode: usize,
    // Motor usage stats (for HUD): aggregated over agent-steps this episode
    pub total_agent_steps: usize,
    pub avg_speed_accum: f32,
    pub heading_change_accum: f32,
    pub comm_signals: Vec<CommSignal>,       // active broadcast resource signals
    pub comm_fitness_accum: Vec<f32>,        // per-agent communication reward accumulation
}

impl Episode {
    pub fn new<R: Rng>(rng: &mut R, agent_count: usize, species_map: &[usize]) -> Self {
        let mut agents = Vec::with_capacity(agent_count);
        for i in 0..agent_count {
            // Uniform random spawn across the world for all agents (no species clustering)
            let pos = world::rand_pos(rng);
            agents.push(Agent {
                id: AgentId(i),
                body: Body { pos, vel: Vec2::new(0.0, 0.0), radius: AGENT_RADIUS },
                theta: -std::f32::consts::FRAC_PI_2,
                energy: crate::params::get_initial_energy().min(crate::params::get_max_energy()),
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
                species_id: *species_map.get(i).unwrap_or(&0),
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
            });
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
            total_agent_steps: 0,
            avg_speed_accum: 0.0,
            heading_change_accum: 0.0,
            comm_signals: Vec::new(),
            comm_fitness_accum: vec![0.0; agent_count],
        }
    }
    pub fn step<R: Rng>(&mut self, population: &[Genome], rng: &mut R) -> bool {
        if self.agents.iter().all(|a| a.energy <= 0.0 || a.health <= DEATH_HEALTH_THRESHOLD) { return false; }
        // Build species ids snapshot (already stored on agent)
        let species_ids: Vec<usize> = self.agents.iter().map(|a| a.species_id).collect();
        // Tick once using shared engine
        let stats: StepDelta = tick_step(
            population,
            &mut self.food,
            &mut self.food_lifetime,
            &mut self.agents,
            &species_ids,
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
