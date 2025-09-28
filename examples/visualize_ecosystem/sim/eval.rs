use std::collections::VecDeque;

use macroquad::prelude::Vec2;
use neat::{genome::Genome, speciator::Speciator};

use crate::{body::Body, params::{AGENT_RADIUS, *}, sensing, sim::*, world::{self, wrap_to_world}};

pub fn eval_population_single_episode(population: &[Genome]) -> Vec<f32> {
    let mut rng = ::rand::rng();
    let mut food = world::build_world(&mut rng);
    // Lightweight speciation for evaluation to provide species differentiation signal
    let mut temp_speciator = Speciator::new(1.0);
    temp_speciator.speciate(population);
    let species_map = build_species_map(&temp_speciator, population.len());
    let mut agents: Vec<Agent> = population.iter().enumerate().map(|(i, _)| Agent {
        id: AgentId(i),
        //pos: world::rand_pos(&mut rng),
        //vel: Vec2 { x: 0.0, y: 0.0 },
        body: Body { 
            pos: world::rand_pos(&mut rng), 
            vel: Vec2::new(0.0, 0.0),
            radius: AGENT_RADIUS,
        },
        theta: -std::f32::consts::FRAC_PI_2,
        energy: INITIAL_ENERGY,
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
        species_id: *species_map.get(i).unwrap_or(&0),
        pooled_plant: [0.0;3], pooled_same: [0.0;3], pooled_other: [0.0;3], pooled_wall: [0.0;3],
        call_intensity: 0.0, heard_sectors: [0.0;3],
    }).collect();
    // Track exploration (unique grid cells); shaping buckets removed for simplification
    let mut visited: Vec<std::collections::HashSet<u32>> = vec![std::collections::HashSet::new(); agents.len()];
    // Communication: active signals + reward accumulators
    let mut signals: Vec<CommSignal> = Vec::new();
    let mut comm_fit: Vec<f32> = vec![0.0; agents.len()];

    let mut steps = 0usize;
    // (Removed approach/spin shaping trackers)
    while steps < MAX_STEPS {
        if agents.iter().all(|a| a.energy <= 0.0 || a.health <= DEATH_HEALTH_THRESHOLD) { break; }
        // Snapshot for predation decisions to avoid borrow conflicts
        // Extended snapshot: (pos, alive, consumed_flag, species_id, is_corpse)
    let species_ids: Vec<usize> = species_map.clone();
        let snapshot: Vec<(Vec2, bool, bool, usize, bool)> = agents.iter().enumerate().map(|(i,a)| {
            let alive = a.energy > 0.0; let is_corpse = !alive && !a.consumed && a.corpse_energy > 0.1;
            (a.body.pos, alive, a.consumed, *species_ids.get(i).unwrap_or(&0), is_corpse)
        }).collect();
        let mut prey_targets: Vec<Option<usize>> = vec![None; agents.len()];
        for (i, a) in agents.iter_mut().enumerate() {
            if a.energy <= 0.0 || a.health <= DEATH_HEALTH_THRESHOLD { continue; }
            // Build extended inputs (ray-first): per-ray signals + energy + memory + density
            let (cur_fx, cur_fy) = sensing::food_vector_from_rays(a.body.pos, a.theta, &food);
            // Update pooled sensing smoothing
            let pools = sensing::compute_sector_pools(a.body.pos, a.theta, &food, &snapshot, i, a.species_id);
            for si in 0..3 { let alpha = POOL_EMA_ALPHA; a.pooled_plant[si] = a.pooled_plant[si] + alpha * (pools.plant_carc[si] - a.pooled_plant[si]); }
            for si in 0..3 { let alpha = POOL_EMA_ALPHA; a.pooled_same[si]  = a.pooled_same[si]  + alpha * (pools.same_alive[si]  - a.pooled_same[si]); }
            for si in 0..3 { let alpha = POOL_EMA_ALPHA; a.pooled_other[si] = a.pooled_other[si] + alpha * (pools.other_alive[si] - a.pooled_other[si]); }
            for si in 0..3 { let alpha = POOL_EMA_ALPHA; a.pooled_wall[si]  = a.pooled_wall[si]  + alpha * (pools.wall[si]       - a.pooled_wall[si]); }
            let density = sensing::density_sectors(a.body.pos, a.theta, &snapshot, i);
            // Digest before acting (shared)
            sim::apply_digestion(a);
            visited[i].insert(grid_index(a.body.pos));
            let energy_in = (a.energy / INITIAL_ENERGY).clamp(0.0, 1.0);
            let my_species = a.species_id;
            let mut inputs = sensing::build_inputs(a.body.pos, a.theta, &food, energy_in, a.last_food_mem, a.last_danger_mem, &density, &snapshot, i, my_species, a.heard_sectors);
            mask_inputs(&mut inputs);
            let out = population[i].evaluate_slice(&inputs);
            // Movement + communication outputs: [turn, thrust(or speed), call]
            let mut raw_turn = out.get(0).copied().unwrap_or(0.0);
            let mut raw_thrust = out.get(1).copied().unwrap_or(0.0);
            let mut raw_call = out.get(2).copied().unwrap_or(0.0);
            // motor noise disabled
            raw_turn = raw_turn.clamp(-1.0, 1.0);
            raw_thrust = raw_thrust.clamp(-1.0, 1.0);
            raw_call = raw_call.clamp(-1.0, 1.0);
            a.call_intensity = (raw_call + 1.0) * 0.5;
            let turn_delta = raw_turn * MAX_TURN_PER_STEP;
            a.theta += turn_delta;
            // wrap heading
            while a.theta > std::f32::consts::PI { a.theta -= 2.0 * std::f32::consts::PI; }
            while a.theta <= -std::f32::consts::PI { a.theta += 2.0 * std::f32::consts::PI; }
            let dir = dir_from_theta(a.theta);
            let speed_for_stats: f32; // actual scalar speed (world units/step) for cost/stats
            let prev = a.body.pos;
            if USE_INERTIA {
                // Apply drag
                a.body.vel *= 1.0 - DRAG_COEFF;

                // Compute thrust
                let thrust_scalar = (raw_thrust + 1.0) * 0.5;
                let mut dv = dir * (thrust_scalar * MAX_THRUST);

                // Braking logic
                if raw_thrust < 0.0 {
                    let forward_speed = a.body.vel.dot(dir);
                    if forward_speed > 0.0 { 
                        let brake = (-raw_thrust).min(1.0) * MAX_THRUST; 
                        dv += dir * -brake; 
                    }
                }

                // Update velocity
                a.body.vel += dv;

                let vlen = a.body.vel.length(); 
                if vlen > MAX_VELOCITY { 
                    a.body.vel *= MAX_VELOCITY / vlen; 
                }

                // Update pos then wrap
                a.body.pos += a.body.vel;
                a.body.pos = wrap_to_world(a.body.pos);

            } else {
                // No inertia
                let mut speed = (raw_thrust + 1.0) * 0.5; 
                if speed > 1.0 { speed = 1.0; }
                let vel = dir * (speed * MAX_SPEED);

                // Update pos then wrap
                a.body.pos += vel;
                a.body.pos = wrap_to_world(a.body.pos);
            }
            let ate = if world::eat_along_path(&mut food, prev, a.body.pos) || world::eat_if_near(&mut food, a.body.pos) {
                if DIGEST_STEPS_PLANT > 0 { a.digest.push_back(DigestEvent { remaining: DIGEST_STEPS_PLANT, per_step: FOOD_ENERGY / (DIGEST_STEPS_PLANT as f32) }); }
                else { a.energy = (a.energy + FOOD_ENERGY).min(INITIAL_ENERGY); }
                a.eaten += 1; true } else { false };
            // Predation/scavenging: choose a nearby target (record only)
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
            // Count survival time
            a.alive_steps += 1;
            // Energy cost: base + movement/velocity + turn + call
            let mut energy_cost = ENERGY_DRAIN_PER_STEP;
            if USE_INERTIA {
                let vmag = a.body.vel.length();
                energy_cost += vmag * EXTRA_VEL_ENERGY_C1 + vmag*vmag*vmag * EXTRA_VEL_ENERGY_C2;
            } else {
                energy_cost += speed_for_stats / MAX_SPEED * MOVE_ENERGY_SCALE;
            }
            let turn_fraction = (raw_turn.abs()).min(1.0);
            if turn_fraction > 0.0 && speed_for_stats > 1e-4 { energy_cost += TURN_ENERGY_SCALE * turn_fraction; }
            // Call cost (scaled by intensity)
            energy_cost += a.call_intensity * CALL_COST;
            a.energy -= energy_cost;
            if a.energy <= 0.0 || a.health <= DEATH_HEALTH_THRESHOLD { if a.dead_since.is_none() { a.dead_since = Some(steps); a.corpse_energy = CORPSE_INITIAL_ENERGY; } }
            // Passive heal based on energy reserve
            if a.energy > 0.0 && a.health > DEATH_HEALTH_THRESHOLD {
                let energy_frac = (a.energy / INITIAL_ENERGY).clamp(0.0,1.0);
                a.health = (a.health + INJURY_HEAL_RATE * energy_frac * a.max_health).min(a.max_health);
            }
            // Update memories after acting
            a.last_food_mem = Vec2 { x: cur_fx, y: cur_fy };
            let (dx_mem, dy_mem) = sensing::nearest_agent_vector_local(a.body.pos, a.theta, &snapshot, i);
            a.last_danger_mem = Vec2 { x: dx_mem, y: dy_mem };
            // Memory decay
            a.last_food_mem.x *= MEMORY_DECAY; a.last_food_mem.y *= MEMORY_DECAY;
            a.last_danger_mem.x *= MEMORY_DECAY; a.last_danger_mem.y *= MEMORY_DECAY;
            // (Removed approach reward & spin penalty accumulation)
            // Communication: record a resource signal if call exceeds threshold and local food cluster
            if a.call_intensity >= COMM_SIGNAL_THRESHOLD {
                // Count nearby food items
                let mut nearby_food = 0usize;
                for f in &food {
                    let dx = f.x - a.body.pos.x; let dy = f.y - a.body.pos.y; if dx*dx + dy*dy <= COMM_FOOD_RADIUS*COMM_FOOD_RADIUS { nearby_food += 1; if nearby_food >= COMM_FOOD_MIN { break; } }
                }
                if nearby_food >= COMM_FOOD_MIN {
                    signals.push(CommSignal { caller: i, pos: a.body.pos, ttl: COMM_SIGNAL_WINDOW });
                }
            }
            // Eating attribution to prior signals (excluding self unless we allow self-benefit?)
            if ate {
                for s in &signals {
                    let dx = a.body.pos.x - s.pos.x; let dy = a.body.pos.y - s.pos.y; if dx*dx + dy*dy <= COMM_SIGNAL_EFFECT_RADIUS*COMM_SIGNAL_EFFECT_RADIUS {
                        if s.caller != i { comm_fit[i] += COMM_RECV_REWARD; }
                        comm_fit[s.caller] += COMM_CALLER_REWARD;
                    }
                }
            }
        }
        // Decay signal TTL and remove expired
        for sig in &mut signals { if sig.ttl > 0 { sig.ttl -= 1; } }
        signals.retain(|s| s.ttl > 0);
        // Resolve predation and tick corpse/flash decay (shared)
        sim::resolve_predation(&mut agents, &prey_targets, steps);
        sim::decay_corpses_and_flashes(&mut agents);
        // Update hearing after all call intensities set
        sensing::update_hearing(&mut agents);
        // (Removed avoidance/crowding penalty sampling)
        // Plants grow/spread over time (season-aware)
        world::set_current_step(steps);
        world::food_growth_step(&mut food, &mut rng);
        steps += 1;
    }

    // Compose final fitness with optional normalized exploration and sublinear eaten term
    let total_cells = ((WORLD_W / EXPL_CELL_SIZE).ceil() * (WORLD_H / EXPL_CELL_SIZE).ceil()) as f32;
    agents.iter().enumerate().map(|(i, a)| {
        let eaten_plants = a.eaten.saturating_sub(a.kills) as f32;
        let eaten_meat = a.kills as f32;
        let intake = eaten_plants * PLANT_FITNESS + eaten_meat * MEAT_FITNESS;
        let frac = if total_cells > 0.0 { (visited[i].len() as f32) / total_cells } else { 0.0 };
        let exploration = frac * EXPL_WEIGHT;
        let survival = (a.alive_steps as f32).powf(SURVIVAL_TIME_EXP) * SURVIVAL_STEP_FITNESS;
        intake + exploration + survival + comm_fit[i]
    }).collect()
}

pub fn build_species_map(speciator: &Speciator, pop_len: usize) -> Vec<usize> {
    let mut map = vec![0usize; pop_len];
    for (sidx, s) in speciator.get_species().iter().enumerate() {
        for &m in &s.members { if m < pop_len { map[m] = sidx; } }
    }
    map
}
