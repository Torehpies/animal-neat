use std::collections::HashSet;

use macroquad::prelude::Vec2;
use neat::genome::Genome;
use ::rand::Rng;
use sim::{resolve_predation, decay_corpses_and_flashes};

use crate::{params::*, sensing, sim::{self, CommSignal, Agent, DigestEvent, dir_from_theta, grid_index}, world::{self, wrap_to_world}};
use crate::body::{resolve_collision, Body};

// Minimal stats delta accumulated during a single tick across all agents
pub struct StepDelta {
    pub total_agent_steps: usize,
    pub avg_speed_accum: f32,
    pub heading_change_accum: f32,
}

impl StepDelta {
    pub fn zero() -> Self { Self { total_agent_steps: 0, avg_speed_accum: 0.0, heading_change_accum: 0.0 } }
}

// Advance the world by a single step for both headless eval and live episode.
// - population: genomes aligned with agents
// - food/agents: mutated in-place
// - species_ids: per-agent species index for same/other pooling distinctions
// - visited: optional exploration tracking (eval only)
// - comm_signals/comm_fit: shared communication state and rewards
// - first_eat_step: optional hook to record first plant eat time (live only)
pub fn tick_step<R: Rng>(
    population: &[Genome],
    food: &mut Vec<Vec2>,
    agents: &mut [Agent],
    species_ids: &[usize],
    mut visited: Option<&mut [HashSet<u32>]>,
    comm_signals: &mut Vec<CommSignal>,
    comm_fit: &mut [f32],
    step_idx: usize,
    rng: &mut R,
    mut first_eat_step: Option<&mut Option<usize>>,
) -> StepDelta {
    // Build snapshot for predation/scavenge decisions
    let snapshot: Vec<(Vec2, bool, bool, usize, bool)> = agents.iter().enumerate().map(|(i,a)| {
        let alive = a.energy > 0.0 && a.health > DEATH_HEALTH_THRESHOLD;
        let is_corpse = !alive && !a.consumed && a.corpse_energy > 0.1;
        (a.body.pos, alive, a.consumed, *species_ids.get(i).unwrap_or(&0), is_corpse)
    }).collect();

    let mut prey_targets: Vec<Option<usize>> = vec![None; agents.len()];
    let mut delta = StepDelta::zero();

    for (i, a) in agents.iter_mut().enumerate() {
        if a.energy <= 0.0 || a.health <= DEATH_HEALTH_THRESHOLD { continue; }

        // Food vector (for memory after acting)
        let (cur_fx, cur_fy) = sensing::food_vector_from_rays(a.body.pos, a.theta, food);

        // Sector pools smoothing
        let pools = sensing::compute_sector_pools(a.body.pos, a.theta, food, &snapshot, i, a.species_id);
        for si in 0..3 { let alpha = POOL_EMA_ALPHA; a.pooled_plant[si] = a.pooled_plant[si] + alpha * (pools.plant_carc[si] - a.pooled_plant[si]); }
        for si in 0..3 { let alpha = POOL_EMA_ALPHA; a.pooled_same[si]  = a.pooled_same[si]  + alpha * (pools.same_alive[si]  - a.pooled_same[si]); }
        for si in 0..3 { let alpha = POOL_EMA_ALPHA; a.pooled_other[si] = a.pooled_other[si] + alpha * (pools.other_alive[si] - a.pooled_other[si]); }
        for si in 0..3 { let alpha = POOL_EMA_ALPHA; a.pooled_wall[si]  = a.pooled_wall[si]  + alpha * (pools.wall[si]       - a.pooled_wall[si]); }

        // Density sectors
        let density = sensing::density_sectors(a.body.pos, a.theta, &snapshot, i);

        // Digest prior energy deliveries
        sim::apply_digestion(a);

        // Exploration map (eval only)
        if let Some(v) = visited.as_deref_mut() { v[i].insert(grid_index(a.body.pos)); }

        let energy_in = (a.energy / INITIAL_ENERGY).clamp(0.0, 1.0);
        let my_species = a.species_id;
        let mut inputs = sensing::build_inputs(
            a.body.pos, a.theta, food, energy_in, a.last_food_mem, a.last_danger_mem, &density, &snapshot, i, my_species, a.heard_sectors
        );
        sim::mask_inputs(&mut inputs);

        let out = population[i].evaluate_slice(&inputs);
        let mut raw_turn = out.get(0).copied().unwrap_or(0.0);
        let mut raw_thrust = out.get(1).copied().unwrap_or(0.0);
    let mut raw_call = out.get(2).copied().unwrap_or(0.0);
        raw_turn = raw_turn.clamp(-1.0, 1.0);
        raw_thrust = raw_thrust.clamp(-1.0, 1.0);
        raw_call = raw_call.clamp(-1.0, 1.0);
    a.call_intensity = if COMMUNICATION_ENABLED { (raw_call + 1.0) * 0.5 } else { 0.0 };
        let turn_delta = raw_turn * MAX_TURN_PER_STEP;
        a.theta += turn_delta;
        while a.theta > std::f32::consts::PI { a.theta -= 2.0 * std::f32::consts::PI; }
        while a.theta <= -std::f32::consts::PI { a.theta += 2.0 * std::f32::consts::PI; }
        let dir = dir_from_theta(a.theta);

        let prev = a.body.pos;
        if USE_INERTIA {
            a.body.vel *= 1.0 - DRAG_COEFF;
            let thrust_scalar = (raw_thrust + 1.0) * 0.5;
            let mut dv = dir * (thrust_scalar * MAX_THRUST);
            if raw_thrust < 0.0 {
                let forward_speed = a.body.vel.dot(dir);
                if forward_speed > 0.0 {
                    let brake = (-raw_thrust).min(1.0) * MAX_THRUST;
                    dv += dir * -brake;
                }
            }
            a.body.vel += dv;
            let vlen = a.body.vel.length();
            if vlen > MAX_VELOCITY { a.body.vel *= MAX_VELOCITY / vlen; }
            a.body.pos += a.body.vel;
            a.body.pos = wrap_to_world(a.body.pos);
        } else {
            let mut speed = (raw_thrust + 1.0) * 0.5;
            if speed > 1.0 { speed = 1.0; }
            let vel = dir * (speed * MAX_SPEED);
            a.body.pos += vel;
            a.body.pos = wrap_to_world(a.body.pos);
        }

        // Continuous eat along path; fallback to body collision at end
        let ate = if world::eat_along_path(food, prev, a.body.pos, a.body.radius) || world::eat_if_near(food, &a.body) {
            if DIGEST_STEPS_PLANT > 0 { a.digest.push_back(DigestEvent { remaining: DIGEST_STEPS_PLANT, per_step: FOOD_ENERGY / (DIGEST_STEPS_PLANT as f32) }); }
            else { a.energy = (a.energy + FOOD_ENERGY).min(INITIAL_ENERGY); }
            a.eaten += 1;
            if let Some(ref mut hook) = first_eat_step { if hook.is_none() { **hook = Some(step_idx); } }
            true
        } else { false };

        // Predation/scavenge candidate
        if PREDATION_ENABLED || SCAVENGE_ENABLED {
            let mut target: Option<usize> = None;
            for j in 0..snapshot.len() {
                if j == i { continue; }
                let (pos_j, alive_j, consumed_j, _species_j, is_corpse_j) = snapshot[j];
                if consumed_j { continue; }
                if (alive_j && !PREDATION_ENABLED) || ((!alive_j || is_corpse_j) && !SCAVENGE_ENABLED) { continue; }
                let dx = pos_j.x - a.body.pos.x; let dy = pos_j.y - a.body.pos.y;
                if (dx*dx + dy*dy).sqrt() <= EAT_AGENT_RADIUS { target = Some(j); break; }
            }
            prey_targets[i] = target;
        }

        // Stats + survival time
        delta.total_agent_steps += 1;
        if USE_INERTIA { delta.avg_speed_accum += a.body.vel.length() / MAX_SPEED; }
        else { delta.avg_speed_accum += (raw_thrust + 1.0) * 0.5; }
        delta.heading_change_accum += turn_delta.abs();

        a.alive_steps += 1;
        // Energy cost
        let mut energy_cost = ENERGY_DRAIN_PER_STEP;
        if USE_INERTIA {
            let vmag = a.body.vel.length();
            energy_cost += vmag * EXTRA_VEL_ENERGY_C1 + vmag*vmag*vmag * EXTRA_VEL_ENERGY_C2;
            if turn_delta.abs() > 0.0 && vmag > 1e-4 { energy_cost += TURN_ENERGY_SCALE * raw_turn.abs().min(1.0); }
        } else {
            let speed = (raw_thrust + 1.0) * 0.5;
            energy_cost += speed * MOVE_ENERGY_SCALE;
            if turn_delta.abs() > 0.0 && speed > 1e-4 { energy_cost += TURN_ENERGY_SCALE * raw_turn.abs().min(1.0); }
        }
    if COMMUNICATION_ENABLED { energy_cost += a.call_intensity * CALL_COST; }
        a.energy -= energy_cost;
        if a.energy <= 0.0 || a.health <= DEATH_HEALTH_THRESHOLD {
            if a.dead_since.is_none() { a.dead_since = Some(step_idx); a.corpse_energy = CORPSE_INITIAL_ENERGY; }
        }
        // Passive heal
        if a.energy > 0.0 && a.health > DEATH_HEALTH_THRESHOLD {
            let ef = (a.energy / INITIAL_ENERGY).clamp(0.0,1.0);
            a.health = (a.health + INJURY_HEAL_RATE * ef * a.max_health).min(a.max_health);
        }

        // Update memories
        a.last_food_mem = Vec2 { x: cur_fx, y: cur_fy };
        let (dx_mem, dy_mem) = sensing::nearest_agent_vector_local(a.body.pos, a.theta, &snapshot, i);
        a.last_danger_mem = Vec2 { x: dx_mem, y: dy_mem };
        a.last_food_mem.x *= MEMORY_DECAY; a.last_food_mem.y *= MEMORY_DECAY;
        a.last_danger_mem.x *= MEMORY_DECAY; a.last_danger_mem.y *= MEMORY_DECAY;

        // Communication: spawn/score signals only when enabled
        if COMMUNICATION_ENABLED {
            if a.call_intensity >= COMM_SIGNAL_THRESHOLD {
                let mut nearby_food = 0usize;
                for f in food.iter() {
                    let dx = f.x - a.body.pos.x; let dy = f.y - a.body.pos.y;
                    if dx*dx + dy*dy <= COMM_FOOD_RADIUS*COMM_FOOD_RADIUS { nearby_food += 1; if nearby_food >= COMM_FOOD_MIN { break; } }
                }
                if nearby_food >= COMM_FOOD_MIN {
                    comm_signals.push(CommSignal { caller: i, pos: a.body.pos, ttl: COMM_SIGNAL_WINDOW });
                }
            }

            if ate {
                for s in comm_signals.iter() {
                    let dx = a.body.pos.x - s.pos.x; let dy = a.body.pos.y - s.pos.y;
                    if dx*dx + dy*dy <= COMM_SIGNAL_EFFECT_RADIUS*COMM_SIGNAL_EFFECT_RADIUS {
                        if s.caller != i { comm_fit[i] += COMM_RECV_REWARD; }
                        comm_fit[s.caller] += COMM_CALLER_REWARD;
                    }
                }
            }
        } else {
            // Ensure hearing inputs decay to zero when communication is off
            a.heard_sectors = [0.0; 3];
        }
    }

    // Decay/cleanup signals
    if COMMUNICATION_ENABLED {
        for sig in comm_signals.iter_mut() { if sig.ttl > 0 { sig.ttl -= 1; } }
        comm_signals.retain(|s| s.ttl > 0);
    } else {
        comm_signals.clear();
    }

    // Optional: resolve simple physical collisions between alive agents
    if AGENT_COLLISIONS_ENABLED && agents.len() > 1 {
        for _ in 0..AGENT_COLLISION_PASSES {
            let n = agents.len();
            for i in 0..n {
                for j in (i+1)..n {
                    // Split borrow for two distinct agents
                    let (left, right) = agents.split_at_mut(j);
                    let a = &mut left[i];
                    let b = &mut right[0];
                    // Only separate alive, non-consumed agents
                    let a_alive = a.energy > 0.0 && a.health > DEATH_HEALTH_THRESHOLD && !a.consumed;
                    let b_alive = b.energy > 0.0 && b.health > DEATH_HEALTH_THRESHOLD && !b.consumed;
                    if !a_alive || !b_alive { continue; }
                    // If overlapping, push apart equally
                    if Body::collides(&a.body, &b.body) {
                        resolve_collision(&mut a.body, &mut b.body);
                        a.body.pos = wrap_to_world(a.body.pos);
                        b.body.pos = wrap_to_world(b.body.pos);
                    }
                }
            }
        }
    }

    // Resolve predation, corpse decay, hearing, and plant growth
    resolve_predation(agents, &prey_targets, step_idx);
    decay_corpses_and_flashes(agents);
    if COMMUNICATION_ENABLED { sensing::update_hearing(agents); }
    world::set_current_step(step_idx);
    world::food_growth_step(food, rng);

    delta
}
