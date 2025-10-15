use std::collections::{HashSet, VecDeque};

use macroquad::prelude::Vec2;
use neat::{genome::Genome, speciator::Speciator};

use crate::{body::Body, params::{AGENT_RADIUS, *}, sim::{CommSignal, Agent, AgentId, tick_step}, world};

pub fn eval_population_single_episode(population: &[Genome]) -> Vec<f32> {
    let mut rng = ::rand::rng();
    let mut food = world::build_world(&mut rng);
    let mut food_lifetime = world::init_food_lifetimes(&food, &mut rng);
    // Lightweight speciation for evaluation to provide species differentiation signal
    let mut temp_speciator = Speciator::new(1.0);
    temp_speciator.speciate(population);
    let species_map = build_species_map(&temp_speciator, population.len());
    let mut agents: Vec<Agent> = population.iter().enumerate().map(|(i, _)| Agent {
        id: AgentId(i),
        body: Body { pos: world::rand_pos(&mut rng), vel: Vec2::new(0.0, 0.0), radius: AGENT_RADIUS },
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
        idle_anchor: world::rand_pos(&mut rng),
        idle_steps: 0,
        total_idle_penalty: 0.0,
        input_buf: vec![0.0; crate::params::INPUTS],
        energy_accum: 0.0,
    }).collect();
    // Track exploration (unique grid cells)
    let mut visited: Vec<HashSet<u32>> = vec![HashSet::new(); agents.len()];
    // Communication state per episode
    let mut signals: Vec<CommSignal> = Vec::new();
    let mut comm_fit: Vec<f32> = vec![0.0; agents.len()];

    let mut steps = 0usize;
    // Track average energy: accumulate per-agent energy while alive
    let mut energy_accum: Vec<f32> = vec![0.0; agents.len()];
    while steps < MAX_STEPS {
        if agents.iter().all(|a| a.energy <= 0.0 || a.health <= DEATH_HEALTH_THRESHOLD) { break; }
        // Use shared step logic
        let _stats = tick_step(
            population,
            &mut food,
            &mut food_lifetime,
            &mut agents,
            &species_map,
            Some(&mut visited),
            &mut signals,
            &mut comm_fit,
            steps,
            &mut rng,
            None,
        );
        // Accumulate energy for alive agents this step
        for (i, a) in agents.iter().enumerate() {
            if a.energy > 0.0 && a.health > DEATH_HEALTH_THRESHOLD { energy_accum[i] += a.energy; }
        }
        steps += 1;
    }

    // Compose final fitness with optional normalized exploration and sublinear eaten term
    let total_cells = ((WORLD_W / EXPL_CELL_SIZE).ceil() * (WORLD_H / EXPL_CELL_SIZE).ceil()) as f32;
    agents.iter().enumerate().map(|(i, a)| {
        let eaten_plants = a.eaten.saturating_sub(a.kills) as f32;
        let eaten_meat = a.kills as f32;
        let intake_events = a.eaten as usize; // total edible events (plants + meat)
        let missing = INTAKE_MIN_EVENTS.saturating_sub(intake_events) as f32;
        let intake_penalty = missing * INTAKE_MISS_PENALTY;
        let intake = eaten_plants * PLANT_FITNESS + eaten_meat * MEAT_FITNESS - intake_penalty;
        let frac = if total_cells > 0.0 { (visited[i].len() as f32) / total_cells } else { 0.0 };
        let exploration = frac * EXPL_WEIGHT;
        let survival = (a.alive_steps as f32).powf(SURVIVAL_TIME_EXP) * SURVIVAL_STEP_FITNESS;
        let predation_reward = (a.attack_hits as f32) * ATTACK_HIT_FITNESS + (a.kills_caused as f32) * KILL_CAUSED_FITNESS;
        // Average energy while alive (normalized 0..1)
        let avg_energy_norm = if a.alive_steps > 0 { (energy_accum[i] / a.alive_steps as f32) / crate::params::get_max_energy() } else { 0.0 };
        let energy_term = avg_energy_norm * ENERGY_AVG_WEIGHT;
        // Subtract idle penalty
        let idle_penalty = a.total_idle_penalty;
        intake + exploration + survival + predation_reward + energy_term + comm_fit[i] - idle_penalty
    }).collect()
}

pub fn build_species_map(speciator: &Speciator, pop_len: usize) -> Vec<usize> {
    let mut map = vec![0usize; pop_len];
    for (sidx, s) in speciator.get_species().iter().enumerate() {
        for &m in &s.members { if m < pop_len { map[m] = sidx; } }
    }
    map
}
