use std::collections::HashSet;

use macroquad::prelude::Vec2;
use neat::genome::Genome;
use ::rand::Rng;
use sim::{resolve_predation, decay_corpses_and_flashes};

use crate::{params::*, sensing, sim::{self, CommSignal, Agent, DigestEvent, dir_from_theta, grid_index}, world::{self, wrap_to_world}};
use crate::body::{resolve_collision, Body};

// Phase 1 output: neural decision + precomputed inputs we still need in phase 2
#[derive(Clone, Default)]
struct ActionIntent {
    alive: bool,
    raw_turn: f32,
    raw_thrust: f32,
    raw_call: f32,
    turn_delta: f32,
    dir: Vec2,
    cur_food_vec: (f32,f32),
    last_same_vec: (f32,f32),
    last_other_vec: (f32,f32),
}

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
    food_lifetime: &mut Vec<usize>,
    agents: &mut [Agent],
    species_ids: &[usize],
    mut visited: Option<&mut [HashSet<u32>]>,
    comm_signals: &mut Vec<CommSignal>,
    comm_fit: &mut [f32],
    step_idx: usize,
    rng: &mut R,
    mut first_eat_step: Option<&mut Option<usize>>,
) -> StepDelta {
    // PERFORMANCE NOTE (two-phase update):
    // We split per-agent work into:
    //   Phase 1 (parallel, read-only on shared world): build inputs, run neural net, compute intended turn/thrust/call and
    //       gather local sensing vectors needed for memory updates. We avoid mutating agents here except through a collected
    //       ActionIntent vector. This keeps contention low and lets rayon parallelize CPU-heavy network evaluation & sensing.
    //   Phase 2 (sequential): apply digestion, integrate movement, energy accounting, eating, predation candidate scan,
    //       memory decay, communication, and stats accumulation. These steps mutate shared collections (food, comm_signals,
    //       per-agent fields) and are kept sequential for simplicity & correctness. Further optimization could batch some
    //       of these (e.g., collision-free movement) in a second parallel pass if profiling shows Phase 2 dominating.
    // The snapshot & age_snapshot are built once before Phase 1 so both phases have a consistent view of other agents.
    // Build snapshot for predation/scavenge decisions
    let snapshot: Vec<(Vec2, bool, bool, usize, bool)> = agents.iter().enumerate().map(|(i,a)| {
        let alive = a.energy > 0.0 && a.health > DEATH_HEALTH_THRESHOLD;
        let is_corpse = !alive && !a.consumed && a.corpse_energy > 0.1;
        (a.body.pos, alive, a.consumed, *species_ids.get(i).unwrap_or(&0), is_corpse)
    }).collect();
    // Age snapshot previously used for newborn grace; no longer needed

    let mut prey_targets: Vec<Option<usize>> = vec![None; agents.len()];
    let mut delta = StepDelta::zero();

    // Phase 1: compute neural outputs in parallel without mutating shared global state (except internal agent fields we copy after)
    // Use food directly (no cloning per step) now that phase 1 is sequential.
    let intents: Vec<ActionIntent> = agents.iter().enumerate().map(|(i, a)| {
        if a.energy <= 0.0 || a.health <= DEATH_HEALTH_THRESHOLD {
            return ActionIntent { alive: false, ..Default::default() };
        }
        let (cur_fx, cur_fy) = sensing::food_vector_from_rays(a.body.pos, a.theta, food);
        let energy_in = (a.energy / crate::params::get_max_energy()).clamp(0.0, 1.0);
        let my_species = a.species_id;
        // Reuse agent's scratch input buffer in-place
    let mut scratch = [0.0f32; crate::params::INPUTS];
        // We can't mutably borrow agent inside map easily; copy inputs then reuse in Phase 2
        // (Simpler sequential Phase 1: we'll fill a temporary and store outputs only.)
        // NOTE: For full reuse, Phase 1 would need mutable access; current iterator borrows immutably.
        // Vision filtering: build local candidate index lists (food + agents) using same CELL grid as predation but larger range for vision
        // Reuse predation grid if already built; for simplicity rebuild lightweight vision lists here (small overhead compared to O(N)).
        const VISION_CELL: f32 = VISION_RANGE / 3.0; // coarser grid for vision
        let gx = (a.body.pos.x / VISION_CELL).floor() as i32; let gy = (a.body.pos.y / VISION_CELL).floor() as i32;
        // Collect food candidates
        let mut food_candidates: Vec<usize> = Vec::new();
        for (fi, fpos) in food.iter().enumerate() {
            // Quick coarse filter by cell distance (cheap); could use a food grid for larger counts
            let fgx = (fpos.x / VISION_CELL).floor() as i32; let fgy = (fpos.y / VISION_CELL).floor() as i32;
            if (fgx - gx).abs() <= 1 && (fgy - gy).abs() <= 1 { food_candidates.push(fi); }
        }
        // Agent snapshot candidates
        let mut agent_candidates: Vec<usize> = Vec::new();
        for (sj, (p, _alive, _consumed, _sid, _corpse)) in snapshot.iter().enumerate() {
            let agx = (p.x / VISION_CELL).floor() as i32; let agy = (p.y / VISION_CELL).floor() as i32;
            if (agx - gx).abs() <= 1 && (agy - gy).abs() <= 1 { agent_candidates.push(sj); }
        }
        sensing::build_inputs_inplace(&mut scratch, a.body.pos, a.theta, food, energy_in, a.last_food_mem, a.last_same_mem, a.last_other_mem, &snapshot, i, my_species, a.heard_sectors, Some(&food_candidates), Some(&agent_candidates));
        sim::mask_inputs(&mut scratch);
        let out = population[i].evaluate_slice(&scratch);
        let mut raw_turn = out.get(0).copied().unwrap_or(0.0);
        let mut raw_thrust = out.get(1).copied().unwrap_or(0.0);
        let mut raw_call = if COMMUNICATION_ENABLED { out.get(2).copied().unwrap_or(0.0) } else { 0.0 };
        raw_turn = raw_turn.clamp(-1.0,1.0);
        raw_thrust = raw_thrust.clamp(-1.0,1.0);
        raw_call = raw_call.clamp(-1.0,1.0);
        let turn_delta = raw_turn * MAX_TURN_PER_STEP;
        // We DON'T mutate agent theta here; just compute dir after hypothetical turn
        let mut theta = a.theta + turn_delta;
        while theta > std::f32::consts::PI { theta -= 2.0 * std::f32::consts::PI; }
        while theta <= -std::f32::consts::PI { theta += 2.0 * std::f32::consts::PI; }
        let dir = dir_from_theta(theta);
        // nearest same/other local vectors (for memory update)
        let ((same_x, same_y), (other_x, other_y)) = sensing::nearest_same_other_vectors_local(a.body.pos, a.theta, &snapshot, i, my_species);
        ActionIntent {
            alive: true,
            raw_turn, raw_thrust, raw_call,
            turn_delta, dir,
            cur_food_vec: (cur_fx, cur_fy),
            last_same_vec: (same_x, same_y),
            last_other_vec: (other_x, other_y),
        }
    }).collect();

    // Build a simple spatial hash (uniform grid) for predation neighbor lookup (alive or corpse energy targets only)
    // Grid cell size tuned to predation radius so we only check local buckets.
    const CELL: f32 = EAT_AGENT_RADIUS * 1.25; // a little larger to capture neighbors
    let mut grid: std::collections::HashMap<(i32,i32), Vec<usize>> = std::collections::HashMap::with_capacity(agents.len()*2);
    for (idx,a) in agents.iter().enumerate() {
        let alive = a.energy > 0.0 && a.health > DEATH_HEALTH_THRESHOLD;
        let is_corpse = !alive && !a.consumed && a.corpse_energy > 0.1;
        if !alive && !is_corpse { continue; }
        let gx = (a.body.pos.x / CELL).floor() as i32;
        let gy = (a.body.pos.y / CELL).floor() as i32;
        grid.entry((gx,gy)).or_default().push(idx);
    }

    // Phase 2: apply decisions sequentially (handles digestion, movement, energy, predation prep, stats, communication)
    for (i, a) in agents.iter_mut().enumerate() {
        if !intents[i].alive { continue; }
        // Digest prior energy
        sim::apply_digestion(a);
        if let Some(v) = visited.as_deref_mut() { v[i].insert(grid_index(a.body.pos)); }
        let intent = &intents[i];
        a.call_intensity = if COMMUNICATION_ENABLED { (intent.raw_call + 1.0) * 0.5 } else { 0.0 };
        a.theta += intent.turn_delta;
        while a.theta > std::f32::consts::PI { a.theta -= 2.0 * std::f32::consts::PI; }
        while a.theta <= -std::f32::consts::PI { a.theta += 2.0 * std::f32::consts::PI; }
        if USE_INERTIA {
            a.body.vel *= 1.0 - DRAG_COEFF;
            let thrust_scalar = (intent.raw_thrust + 1.0) * 0.5;
            let mut dv = intent.dir * (thrust_scalar * MAX_THRUST);
            if intent.raw_thrust < 0.0 {
                let forward_speed = a.body.vel.dot(intent.dir);
                if forward_speed > 0.0 {
                    let brake = (-intent.raw_thrust).min(1.0) * MAX_THRUST;
                    dv += intent.dir * -brake;
                }
            }
            a.body.vel += dv;
        }
        // Movement integration
        if USE_INERTIA {
            let vlen = a.body.vel.length();
            if vlen > MAX_VELOCITY { a.body.vel *= MAX_VELOCITY / vlen; }
            a.body.pos += a.body.vel;
            a.body.pos = wrap_to_world(a.body.pos);
        } else {
            let mut speed = (intent.raw_thrust + 1.0) * 0.5; if speed > 1.0 { speed = 1.0; }
            let vel = intent.dir * (speed * MAX_SPEED);
            a.body.pos += vel; a.body.pos = wrap_to_world(a.body.pos);
        }
        // Idleness
        if IDLENESS_PENALTY_ENABLED {
            let dist_from_anchor = (a.body.pos - a.idle_anchor).length();
            if dist_from_anchor < IDLENESS_DISTANCE_THRESHOLD {
                a.idle_steps += 1;
                a.total_idle_steps = a.total_idle_steps.saturating_add(1);
                // Count idle penalty in unit steps beyond threshold; fitness weight scales impact
                if a.idle_steps > IDLENESS_THRESHOLD_STEPS { a.total_idle_penalty += 1.0; }
            } else { a.idle_anchor = a.body.pos; a.idle_steps = 0; }
        }
        // Eating
    let ate = if world::eat_if_near(food, food_lifetime, &a.body) {
            if DIGEST_STEPS_PLANT > 0 { a.digest.push_back(DigestEvent { remaining: DIGEST_STEPS_PLANT, per_step: FOOD_ENERGY / (DIGEST_STEPS_PLANT as f32) }); }
            else { a.energy = (a.energy + FOOD_ENERGY).min(crate::params::get_max_energy()); }
            a.eaten += 1; if let Some(ref mut hook) = first_eat_step { if hook.is_none() { **hook = Some(step_idx); } } true } else { false };
        // Predation candidate scan via spatial grid
        if PREDATION_ENABLED || SCAVENGE_ENABLED {
            let dir = intent.dir; let mut target: Option<usize> = None;
            let gx = (a.body.pos.x / CELL).floor() as i32; let gy = (a.body.pos.y / CELL).floor() as i32;
            let half_cone_cos = (VISION_ANGLE_DEG.to_radians() * 0.5).cos();
            // Check surrounding 3x3 cells
            'outer: for oy in -1..=1 { for ox in -1..=1 { if let Some(bucket) = grid.get(&(gx+ox, gy+oy)) {
                for &j in bucket { if j == i { continue; }
                    let (pos_j, alive_j, consumed_j, _species_j, is_corpse_j) = snapshot[j]; if consumed_j { continue; }
                    // Previously: kinship protection skipped attacking same-species or genetically similar agents.
                    // Now removed: agents may attack any valid target regardless of genetic similarity or species.
                    if (alive_j && !PREDATION_ENABLED) || ((!alive_j || is_corpse_j) && !SCAVENGE_ENABLED) { continue; }
                    let dx = pos_j.x - a.body.pos.x; let dy = pos_j.y - a.body.pos.y; let dist2 = dx*dx + dy*dy; if dist2 > EAT_AGENT_RADIUS*EAT_AGENT_RADIUS { continue; }
                    if alive_j && PREDATION_REQUIRES_VISION { let len = (dist2 as f32).sqrt(); if len < 1e-6 { continue; } let dot = dir.dot(Vec2::new(dx,dy)/len); if dot < half_cone_cos { continue; } }
                    if (!alive_j || is_corpse_j) && SCAVENGE_REQUIRES_VISION { let len = (dist2 as f32).sqrt(); if len < 1e-6 { continue; } let dot = dir.dot(Vec2::new(dx,dy)/len); if dot < half_cone_cos { continue; } }
                    target = Some(j); break 'outer;
                }
            } }}
            prey_targets[i] = target;
        }
        // Stats & energy
        delta.total_agent_steps += 1;
        if USE_INERTIA { delta.avg_speed_accum += a.body.vel.length() / MAX_SPEED; }
        else { delta.avg_speed_accum += (intent.raw_thrust + 1.0) * 0.5; }
        delta.heading_change_accum += intent.turn_delta.abs();
        a.alive_steps += 1; a.age_steps = a.age_steps.saturating_add(1);
        let mut energy_cost = crate::params::get_energy_drain_per_step();
        if USE_INERTIA { let vmag = a.body.vel.length(); energy_cost += vmag * EXTRA_VEL_ENERGY_C1 + vmag*vmag*vmag * EXTRA_VEL_ENERGY_C2; if intent.turn_delta.abs()>0.0 && vmag>1e-4 { energy_cost += TURN_ENERGY_SCALE * intent.raw_turn.abs().min(1.0); } }
        else { let speed = (intent.raw_thrust + 1.0) * 0.5; energy_cost += speed * MOVE_ENERGY_SCALE; if intent.turn_delta.abs()>0.0 && speed>1e-4 { energy_cost += TURN_ENERGY_SCALE * intent.raw_turn.abs().min(1.0); } }
        if COMMUNICATION_ENABLED { energy_cost += a.call_intensity * CALL_COST; }
        a.energy -= energy_cost; if a.energy <= 0.0 || a.health <= DEATH_HEALTH_THRESHOLD { if a.dead_since.is_none() { a.dead_since = Some(step_idx); a.corpse_energy = CORPSE_INITIAL_ENERGY; } }
        if a.energy > 0.0 && a.health > DEATH_HEALTH_THRESHOLD { let ef=(a.energy/ crate::params::get_max_energy()).clamp(0.0,1.0); a.health=(a.health + INJURY_HEAL_RATE * ef * a.max_health).min(a.max_health); }
    // Accumulate energy for eco-mode live fitness to avoid extra evaluation pass later
    if a.energy > 0.0 && a.health > DEATH_HEALTH_THRESHOLD { a.energy_accum += a.energy; }
        // Memories (decay after storing new vectors)
        a.last_food_mem = Vec2 { x: intent.cur_food_vec.0, y: intent.cur_food_vec.1 };
        a.last_same_mem = Vec2 { x: intent.last_same_vec.0, y: intent.last_same_vec.1 };
        a.last_other_mem = Vec2 { x: intent.last_other_vec.0, y: intent.last_other_vec.1 };
        a.last_food_mem.x *= MEMORY_DECAY; a.last_food_mem.y *= MEMORY_DECAY;
        a.last_same_mem.x *= MEMORY_DECAY; a.last_same_mem.y *= MEMORY_DECAY;
        a.last_other_mem.x *= MEMORY_DECAY; a.last_other_mem.y *= MEMORY_DECAY;
        // Herding fitness shaping removed for simplification
        if COMMUNICATION_ENABLED {
            if a.call_intensity >= COMM_SIGNAL_THRESHOLD {
                let mut nearby_food=0usize;
                for f in food.iter(){ let dx=f.x - a.body.pos.x; let dy=f.y - a.body.pos.y; if dx*dx + dy*dy <= COMM_FOOD_RADIUS*COMM_FOOD_RADIUS { nearby_food+=1; if nearby_food>=COMM_FOOD_MIN { break; } } }
                if nearby_food>=COMM_FOOD_MIN { comm_signals.push(CommSignal { caller:i,pos:a.body.pos,ttl:COMM_SIGNAL_WINDOW }); }
            }
            if ate {
                // Credit unit rewards for communication-assisted eating; scaled by fitness weight later
                for s in comm_signals.iter(){
                    let dx=a.body.pos.x - s.pos.x; let dy=a.body.pos.y - s.pos.y;
                    if dx*dx + dy*dy <= COMM_SIGNAL_EFFECT_RADIUS*COMM_SIGNAL_EFFECT_RADIUS {
                        if s.caller != i { if let Some(f)=comm_fit.get_mut(i){ *f += 1.0; } }
                        if let Some(f)=comm_fit.get_mut(s.caller){ *f += 1.0; }
                    }
                }
            }
        } else { a.heard_sectors = [0.0;3]; }
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
    world::food_growth_and_aging_step(food, food_lifetime, rng);

    delta
}
