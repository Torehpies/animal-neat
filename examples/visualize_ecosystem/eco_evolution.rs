use ::rand::Rng;
use macroquad::prelude::*;

use crate::sim::Episode;
use crate::sim::{Agent, AgentId};
use crate::params as params;

use neat::neat::{
    config::EvolutionConfig,
    evolution,
    genome::Genome,
    innovation_tracker::InnovationTracker,
};

/// Finalize the just-completed episode. In continuous eco mode this triggers culling/evolution
/// per-episode; otherwise it runs the generation-based evolve path.
pub fn finalize_end_of_episode(state: &mut crate::AppState, rng: &mut impl ::rand::Rng) {
    use crate::params::ECO_CONTINUOUS;
    // Compute intelligence proxy (from the just-finished episode) before resetting
    // Proxy focuses on behavior-shaping components: herding + approach/chase (other/same)
    let (intel_best, intel_mean) = {
        // Use per-kind behavior shaping weights
        let mut best = f32::NEG_INFINITY;
        let mut sum = 0.0f32;
        let mut count = 0usize;
        for a in &state.episode.agents {
            let kind = match a.kind { crate::sim::AgentKind::Herbivore => crate::params::Kind::Herb, crate::sim::AgentKind::Carnivore => crate::params::Kind::Carn };
            let w_approach = crate::params::get_fit_approach_food_weight(kind);
            let w_chase = crate::params::get_fit_chase_other_weight(kind);
            let w_chase_same = crate::params::get_fit_chase_same_weight(kind);
            let w_herd = crate::params::get_fit_herding_weight(kind);
            let w_flee = crate::params::get_fit_flee_other_weight(kind);
            let w_restc = crate::params::get_fit_rest_content_weight(kind);
            let v = w_herd * a.herding_units
                + w_approach * a.approach_food_units
                + w_chase * a.chase_other_units
                + w_chase_same * a.chase_same_units
                + w_flee * a.flee_other_units;
            if v.is_finite() {
                if v > best { best = v; }
                sum += v; count += 1;
            }
        }
        let mean = if count > 0 { sum / count as f32 } else { 0.0 };
        (best, mean)
    };
    if ECO_CONTINUOUS {
        state.eco_episode_counter += 1;
        eco_cull_population_by_fitness(state);
        // Log fitness (best/mean) for eco mode as well
        if state.last_best.is_finite() { state.graphs.best.push(state.last_best); }
        if state.last_avg.is_finite() { state.graphs.mean.push(state.last_avg); }
        // Log intelligence proxy for this episode
        if intel_best.is_finite() { state.graphs.intel_best.push(intel_best); }
        if intel_mean.is_finite() { state.graphs.intel_mean.push(intel_mean); }
        state.graphs.reset_episode();
    let sc = &state.sim_config;
    crate::params::set_runtime_species_counts(sc.herbivore_count, sc.carnivore_count);
    state.episode = Episode::new(rng, sc.herbivore_count, sc.carnivore_count);
    } else {
        // Log intelligence proxy for this episode (before resetting)
        if intel_best.is_finite() { state.graphs.intel_best.push(intel_best); }
        if intel_mean.is_finite() { state.graphs.intel_mean.push(intel_mean); }
        state.evolve_one_generation();
        state.graphs.reset_episode();
    let sc = &state.sim_config;
    crate::params::set_runtime_species_counts(sc.herbivore_count, sc.carnivore_count);
    state.episode = Episode::new(rng, sc.herbivore_count, sc.carnivore_count);
        if state.last_best.is_finite() { state.graphs.best.push(state.last_best); state.graphs.mean.push(state.last_avg); }
    }
    state.scoreboard_pending = false;
    // Keep panel toggle as the user last set it (don't force-close), but clear the data
    state.scoreboard_rows.clear();
}

/// Cull/evolve population using live-episode fitness scores (used in ECO_CONTINUOUS mode).
pub fn eco_cull_population_by_fitness(state: &mut crate::AppState) {
    // Strict NEAT at episode end using live-episode fitness for ALL agents (including newborns).
    // 1) Compute fitness scores from the finished episode
    let mut scores: Vec<f32> = {
        let complexity_penalty = crate::params::COMPLEXITY_PENALTY_PER_CONN;
        state.episode.agents.iter().enumerate().map(|(i, a)| {
            let kind = match a.kind { crate::sim::AgentKind::Herbivore => crate::params::Kind::Herb, crate::sim::AgentKind::Carnivore => crate::params::Kind::Carn };
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
            let w_approach = crate::params::get_fit_approach_food_weight(kind);
            let w_chase = crate::params::get_fit_chase_other_weight(kind);
            let w_chase_same = crate::params::get_fit_chase_same_weight(kind);
            let w_restc = crate::params::get_fit_rest_content_weight(kind);
            let w_flee = crate::params::get_fit_flee_other_weight(kind);
            let w_eearly = crate::params::get_fit_eat_early_weight(kind);
            let lifetime_score = (a.alive_steps as f32) / (params::MAX_STEPS as f32);
            let avg_energy_norm = if a.alive_steps > 0 { (a.energy_accum / a.alive_steps as f32) / crate::params::get_max_energy_for(kind) } else { 0.0 };
            let offspring_score = a.offspring_count as f32;
            let comm_score = state.episode.comm_fitness_accum.get(i).copied().unwrap_or(0.0);
            // Normalize idle penalty to [0,1] of max achievable this episode
            let max_idle_steps = (params::MAX_STEPS.saturating_sub(params::IDLENESS_THRESHOLD_STEPS)) as f32;
            let max_idle_penalty = max_idle_steps * params::IDLENESS_PENALTY_PER_STEP;
            let idle_penalty = if max_idle_penalty > 0.0 { (a.total_idle_penalty / max_idle_penalty).clamp(0.0, 1.0) } else { 0.0 };
            let plants = a.eaten.saturating_sub(a.kills) as f32;
            let meat = a.kills as f32;
            let approach_units = a.approach_food_units;
            let chase_units = a.chase_other_units;
            let chase_same_units = a.chase_same_units;
            let flee_units = a.flee_other_units;
            let mut s = w_life * lifetime_score
                + w_energy * avg_energy_norm
                + w_off * offspring_score
                + w_comm * comm_score
                - if params::IDLENESS_PENALTY_ENABLED { w_idle * idle_penalty } else { 0.0 }
                + w_plant * plants
                + w_meat * meat
                + w_att * (a.attack_hits as f32)
                + w_kill * (a.kills_caused as f32)
                + w_herd * a.herding_units
                + w_approach * approach_units
                + w_chase * chase_units
                + w_chase_same * chase_same_units
                + w_restc * a.rest_content_units
                + w_flee * flee_units
                + w_eearly * a.eat_early_units;
            if complexity_penalty > 0.0 {
                let enabled = state.population[i].connections.iter().filter(|c| c.enabled).count() as f32;
                s -= complexity_penalty * enabled;
            }
            s
        }).collect()
    };

    // 2) Cull to target population size by global fitness order (if current pop > target)
    let target = crate::params::get_population_size();
    let current_len = state.population.len();
    if current_len > target {
        let mut idxs: Vec<usize> = (0..current_len).collect();
        idxs.sort_by(|&a, &b| scores[b]
            .partial_cmp(&scores[a])
            .unwrap_or(std::cmp::Ordering::Equal));
        idxs.truncate(target);
        // rebuild population and scores in selected order
        let mut new_pop = Vec::with_capacity(idxs.len());
        let mut new_scores = Vec::with_capacity(idxs.len());
        for i in idxs { new_pop.push(state.population[i].clone()); new_scores.push(scores[i]); }
        state.population = new_pop;
        scores = new_scores;
    }

    // 3) Hand off to the library's NEAT evolution with speciation-aware reproduction
    let old_len = state.population.len(); // after culling
    state.population = evolution::evolution(
        std::mem::take(&mut state.population),
        scores.clone(), // aligned 1:1 with culled population
        &mut state.speciator,
        &mut state.innov,
        &state.cfg,
    );
    debug_assert_eq!(state.population.len(), old_len, "population size should be stable across generations");

    // 4) Update stats for HUD
    if !scores.is_empty() {
        let best = scores.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        let avg = scores.iter().sum::<f32>() / (scores.len() as f32);
        state.last_best = best;
        state.last_avg = avg;
        state.last_best_generation = state.eco_episode_counter; // last eco-episode index
        // Snapshot best genome: pick elite after evolution (first of best species kept by evolution)
        if let Some((_idx, _)) = scores
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
        {
            // Note: after evolution, ordering changed; snapshot just uses the new population's first elite of the best species
            state.last_best_genome = state.population.first().cloned();
        }
    }

    // 5) Recompute species and member mapping for the next episode
    state.speciator.speciate(&state.population);
    state.member_species = {
        let mut map = vec![0usize; state.population.len()];
        for (sidx, s) in state.speciator.get_species().iter().enumerate() {
            for &m in &s.members {
                if m < state.population.len() {
                    map[m] = sidx;
                }
            }
        }
        map
    };
}

/// Possibly spawn newborns mid-episode given energy/cooldown/adjacency rules. Returns realized births.
pub fn spawn_offspring_if_needed<R: Rng>(
    population: &mut Vec<Genome>,
    episode: &mut Episode,
    _innov: &mut InnovationTracker,
    _cfg: &EvolutionConfig,
    rng: &mut R,
) -> usize {
    // Count live agents and skip if at cap
    let mut live_indices: Vec<usize> = Vec::new();
    for (i, a) in episode.agents.iter().enumerate() {
        if a.energy > 0.0 && a.health > params::DEATH_HEALTH_THRESHOLD && !a.consumed { live_indices.push(i); }
    }
    if live_indices.len() >= params::ECO_MAX_POP { return 0; }

    // First, tick down cooldowns for all alive agents
    for i in &live_indices {
        let a = &mut episode.agents[*i];
        if a.repro_cooldown > 0 { a.repro_cooldown -= 1; }
    }

    // Gather eligible parents (energy, cooldown, offspring cap, optional non-idle)
    let mut eligible: Vec<usize> = Vec::new();
    for &i in &live_indices {
        let a = &episode.agents[i];
        let non_idle_ok = if params::ECO_REQUIRE_NON_IDLE_FOR_BIRTH {
            a.body.vel.length() > 0.2 || a.idle_steps < params::IDLENESS_THRESHOLD_STEPS
        } else { true };
        if a.repro_cooldown == 0 && a.offspring_count < params::ECO_MAX_OFFSPRING_PER_AGENT && non_idle_ok {
            eligible.push(i);
        }
    }

    // Attempt to find pairs among eligible agents. Only allow mating within the same kind (Herbivore/Carnivore).
    let cost = params::ECO_BIRTH_ENERGY_COST;
    let mut births: Vec<(usize, usize, crate::body::Body)> = Vec::new(); // (p1_idx, p2_idx, child_body)
    let mut used: std::collections::HashSet<usize> = std::collections::HashSet::new();
    eligible.sort_unstable();
    'pairing: for (ii, &i) in eligible.iter().enumerate() {
        if used.contains(&i) { continue; }
        let ai = &episode.agents[i];
        for &j in eligible.iter().skip(ii+1) {
            if used.contains(&j) { continue; }
            let aj = &episode.agents[j];
            // Kind check: must be the same ecological kind
            if ai.kind != aj.kind { continue; }
            // Energy cost feasibility
            let half = cost * 0.5; if ai.energy < half || aj.energy < half { continue; }
            // Require agents to be touching (wrap-aware shortest distance)
            let mut dx = ai.body.pos.x - aj.body.pos.x;
            if dx.abs() > params::WORLD_W * 0.5 {
                if dx > 0.0 { dx -= params::WORLD_W; } else { dx += params::WORLD_W; }
            }
            let mut dy = ai.body.pos.y - aj.body.pos.y;
            if dy.abs() > params::WORLD_H * 0.5 {
                if dy > 0.0 { dy -= params::WORLD_H; } else { dy += params::WORLD_H; }
            }
            let dist2 = dx*dx + dy*dy;
            let touch_r = ai.body.radius + aj.body.radius;
            if dist2 > touch_r * touch_r { continue; }

            // Child spawn near parents: midpoint with small jitter
            let mid = Vec2 { x: (ai.body.pos.x + aj.body.pos.x) * 0.5, y: (ai.body.pos.y + aj.body.pos.y) * 0.5 };
            let jitter = Vec2 { x: (rng.random::<f32>() - 0.5) * 3.0 * params::AGENT_RADIUS, y: (rng.random::<f32>() - 0.5) * 3.0 * params::AGENT_RADIUS };
            let mut pos = mid + jitter;
            pos.x = (pos.x % params::WORLD_W + params::WORLD_W) % params::WORLD_W; pos.y = (pos.y % params::WORLD_H + params::WORLD_H) % params::WORLD_H;
            births.push((i, j, crate::body::Body { pos, vel: Vec2::new(0.0, 0.0), radius: params::AGENT_COLLISION_RADIUS }));
            used.insert(i); used.insert(j);
            if live_indices.len() + births.len() >= params::ECO_MAX_POP { break 'pairing; }
        }
    }

    if births.is_empty() { return 0; }

    // Realize births: create child via crossover + mutation; debit both parents; set cooldowns; append agent + genome
    let mut realized = 0usize;
    for (i, j, body) in births {
        // Double-check alignment: parents indices map to genome indices
        if i >= population.len() || j >= population.len() { continue; }
        let p1 = &population[i];
        let p2 = &population[j];
        // Offspring are produced by crossover from parents and immediately mutated (per request)
        let mut child_g = neat::neat::crossover::crossover(p1, p2);
        child_g.mutate(_innov, _cfg);
        population.push(child_g);

        // Decide newborn kind: inherit if parents share kind, otherwise random choice
        let child_kind = {
            let ai_kind = episode.agents[i].kind;
            let aj_kind = episode.agents[j].kind;
            // Parents are required to be same kind; inherit directly
            if ai_kind == aj_kind { ai_kind } else { ai_kind }
        };
        // Append newborn agent aligned with last genome
        let birth_pos = body.pos;  // Save position before moving body
        episode.agents.push(Agent {
            id: AgentId(episode.agents.len()),
            kind: child_kind,
            body,
            theta: -std::f32::consts::FRAC_PI_2,
            energy: params::get_offspring_energy(match child_kind { crate::sim::AgentKind::Herbivore => params::Kind::Herb, crate::sim::AgentKind::Carnivore => params::Kind::Carn }).min(params::MAX_ENERGY),
            health: params::ECO_NEWBORN_HEALTH,
            max_health: params::ECO_NEWBORN_HEALTH,
            invuln_steps: 0,
            alive_steps: 0,
            eaten: 0,
            consumed: false,
            kills: 0,
            predation_flash_steps: 0,
            dead_since: None,
            corpse_energy: 0.0,
            digest: std::collections::VecDeque::new(),
            last_food_mem: Vec2 { x: 0.0, y: 0.0 },
            last_danger_mem: Vec2 { x: 0.0, y: 0.0 },
            last_same_mem: Vec2 { x: 0.0, y: 0.0 },
            last_other_mem: Vec2 { x: 0.0, y: 0.0 },
            // For code paths that still read species_id, mirror kind: Herbivore=0, Carnivore=1
            species_id: match child_kind { crate::sim::AgentKind::Herbivore => 0, crate::sim::AgentKind::Carnivore => 1 },
            age_steps: 0,
            call_intensity: 0.0,
            heard_sectors: [0.0;3],
            repro_cooldown: params::get_offspring_cooldown(match child_kind { crate::sim::AgentKind::Herbivore => params::Kind::Herb, crate::sim::AgentKind::Carnivore => params::Kind::Carn }),
            offspring_count: 0,
            attack_hits: 0,
            kills_caused: 0,
            idle_anchor: birth_pos,
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
        // Extend comm fitness accumulator to match agents length
        episode.comm_fitness_accum.push(0.0);
        episode.births_this_episode += 1;
        match child_kind {
            crate::sim::AgentKind::Herbivore => { episode.births_herb += 1; }
            crate::sim::AgentKind::Carnivore => { episode.births_carn += 1; }
        }

        // Apply costs and cooldowns to parents; then push them apart to reduce clustering after birth
        {
            let (a_idx, b_idx) = if i < j { (i, j) } else { (j, i) };
            let (left, right) = episode.agents.split_at_mut(b_idx);
            let pa = &mut left[a_idx];
            let pb = &mut right[0];
            pa.energy = (pa.energy - cost * 0.5).max(0.0);
            pb.energy = (pb.energy - cost * 0.5).max(0.0);
            pa.offspring_count += 1;
            pb.offspring_count += 1;
            // Scale cooldown by current offspring count to space repeated births (use per-kind cooldown)
            let pa_cooldown_base = params::get_offspring_cooldown(match pa.kind { crate::sim::AgentKind::Herbivore => params::Kind::Herb, crate::sim::AgentKind::Carnivore => params::Kind::Carn });
            let pb_cooldown_base = params::get_offspring_cooldown(match pb.kind { crate::sim::AgentKind::Herbivore => params::Kind::Herb, crate::sim::AgentKind::Carnivore => params::Kind::Carn });
            pa.repro_cooldown = pa_cooldown_base + (pa.offspring_count * (pa_cooldown_base / 2));
            pb.repro_cooldown = pb_cooldown_base + (pb.offspring_count * (pb_cooldown_base / 2));
            // Separation impulse to reduce clustering after birth
            let sep_ab = pa.body.pos - pb.body.pos;
            let len = sep_ab.length();
            if len > 1e-3 {
                let dir = sep_ab / len;
                pa.body.vel += dir * params::BIRTH_SEPARATION_IMPULSE;
                pb.body.vel -= dir * params::BIRTH_SEPARATION_IMPULSE;
            }
        }
        // Award unit reproduction credit to parents; scaled by fitness weights later
        if let Some(fit) = episode.comm_fitness_accum.get_mut(i) { *fit += 1.0; }
        if let Some(fit) = episode.comm_fitness_accum.get_mut(j) { *fit += 1.0; }
        realized += 1;
    }
    realized
}
