use std::collections::{HashSet, VecDeque};

use macroquad::prelude::Vec2;
use neat::genome::Genome;

use crate::{body::Body, params::*, sim::{CommSignal, Agent, AgentId, AgentKind, tick_step}, world};

pub fn eval_population_single_episode(population: &[Genome]) -> Vec<f32> {
    let mut rng = ::rand::rng();
    let mut food = world::build_world(&mut rng);
    let mut food_lifetime = world::init_food_lifetimes(&food, &mut rng);
    let mut agents: Vec<Agent> = population.iter().enumerate().map(|(i, _)| Agent {
        id: AgentId(i),
        kind: if i % 2 == 0 { AgentKind::Herbivore } else { AgentKind::Carnivore },
    body: Body { pos: world::rand_pos(&mut rng), vel: Vec2::new(0.0, 0.0), radius: AGENT_COLLISION_RADIUS },
        theta: -std::f32::consts::FRAC_PI_2,
    energy: {
        let k = if i % 2 == 0 { crate::params::Kind::Herb } else { crate::params::Kind::Carn };
        crate::params::get_initial_energy_for(k).min(crate::params::get_max_energy_for(k))
    },
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
    species_id: if i % 2 == 0 { 0 } else { 1 },
        age_steps: 0,
        call_intensity: 0.0, heard_sectors: [0.0;3],
        repro_cooldown: 0,
        offspring_count: 0,
        attack_hits: 0,
        kills_caused: 0,
        idle_anchor: world::rand_pos(&mut rng),
        idle_steps: 0,
            rest_content_units: 0.0,
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
        eat_early_units: 0.0,
        is_eating: false,
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

    let complexity_penalty = crate::params::COMPLEXITY_PENALTY_PER_CONN;
    agents.iter().enumerate().map(|(i, a)| {
        let kind = match a.kind { AgentKind::Herbivore => crate::params::Kind::Herb, AgentKind::Carnivore => crate::params::Kind::Carn };
        // Per-kind behavior shaping weights
        let w_approach = crate::params::get_fit_approach_food_weight(kind);
        let w_chase = crate::params::get_fit_chase_other_weight(kind);
        let w_chase_same = crate::params::get_fit_chase_same_weight(kind);
    let w_restc = crate::params::get_fit_rest_content_weight(kind);
    let w_flee = crate::params::get_fit_flee_other_weight(kind);
        let w_eearly = crate::params::get_fit_eat_early_weight(kind);
        // Per-kind fitness weights
        let w_life = crate::params::get_fit_lifetime_weight(kind);
        let w_energy = crate::params::get_fit_energy_weight(kind);
        let w_off = crate::params::get_fit_offspring_weight(kind);
        let w_comm = crate::params::get_fit_comm_weight(kind);
        let w_idle = crate::params::get_fit_idle_penalty_weight(kind);
        let w_plant = crate::params::get_fit_plant_weight(kind);
        let w_meat = crate::params::get_fit_meat_weight(kind);
        let w_att = crate::params::get_fit_attacks_weight(kind);
        let w_kill = crate::params::get_fit_kills_weight(kind);
        let w_herd = crate::params::get_fit_herding_weight(kind);
        let lifetime_score = (a.alive_steps as f32) / (MAX_STEPS as f32);
    let avg_energy_norm = if a.alive_steps > 0 { (energy_accum[i] / a.alive_steps as f32) / crate::params::get_max_energy_for(kind) } else { 0.0 };
        let offspring_score = a.offspring_count as f32;
        let comm_score = comm_fit.get(i).copied().unwrap_or(0.0);
        // Normalize idle penalty to [0,1] fraction of maximum possible idle accumulation this episode
        let max_idle_steps = (MAX_STEPS.saturating_sub(IDLENESS_THRESHOLD_STEPS)) as f32;
        let max_idle_penalty = max_idle_steps * IDLENESS_PENALTY_PER_STEP;
        let idle_penalty = if max_idle_penalty > 0.0 {
            (a.total_idle_penalty / max_idle_penalty).clamp(0.0, 1.0)
        } else { 0.0 };
        let plants = a.eaten.saturating_sub(a.kills) as f32;
        let meat = a.kills as f32;
        let herd_units = a.herding_units;
        let approach_units = a.approach_food_units;
    let chase_units = a.chase_other_units;
    let chase_same_units = a.chase_same_units;
    let flee_units = a.flee_other_units;
        let mut s = w_life * lifetime_score
            + w_energy * avg_energy_norm
            + w_off * offspring_score
            + w_comm * comm_score
            - if IDLENESS_PENALTY_ENABLED { w_idle * idle_penalty } else { 0.0 }
            + w_plant * plants
            + w_meat * meat
            + w_att * (a.attack_hits as f32)
            + w_kill * (a.kills_caused as f32)
            + w_herd * herd_units
            + w_approach * approach_units
            + w_chase * chase_units
            + w_chase_same * chase_same_units
            + w_restc * a.rest_content_units
            + w_flee * flee_units
            + w_eearly * a.eat_early_units;
        if complexity_penalty > 0.0 {
            // Penalize number of enabled connections in the genome
            let enabled = population[i].connections.iter().filter(|c| c.enabled).count() as f32;
            s -= complexity_penalty * enabled;
        }
        s
    }).collect()
}

