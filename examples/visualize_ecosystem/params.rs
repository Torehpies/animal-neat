// Centralized simulation parameters
//
// This file groups knobs by domain: world, plants, perception, movement,
// ecology, sensing/memory, fitness/shaping, digestion, evolution/speciation.
// Where possible, keep names stable to avoid touching call sites.

use std::cell::Cell;

// Runtime config storage for energy parameters (thread-local)
thread_local! {
    static RUNTIME_INITIAL_ENERGY: Cell<f32> = Cell::new(500.0);
    static RUNTIME_MAX_ENERGY: Cell<f32> = Cell::new(5000.0);
    static RUNTIME_ENERGY_DRAIN: Cell<f32> = Cell::new(0.05);
    static RUNTIME_POPULATION_SIZE: Cell<usize> = Cell::new(50);
}

// Getters for runtime energy config (fallback to these constants if not set)
pub fn get_initial_energy() -> f32 { RUNTIME_INITIAL_ENERGY.with(|c| c.get()) }
pub fn get_max_energy() -> f32 { RUNTIME_MAX_ENERGY.with(|c| c.get()) }
pub fn get_energy_drain_per_step() -> f32 { RUNTIME_ENERGY_DRAIN.with(|c| c.get()) }
pub fn get_population_size() -> usize { RUNTIME_POPULATION_SIZE.with(|c| c.get()) }

// Setter for runtime energy config
pub fn set_runtime_energy_config(initial: f32, max: f32, drain: f32) {
    RUNTIME_INITIAL_ENERGY.with(|c| c.set(initial));
    RUNTIME_MAX_ENERGY.with(|c| c.set(max));
    RUNTIME_ENERGY_DRAIN.with(|c| c.set(drain));
}

// Setter for runtime population size
pub fn set_runtime_population_size(size: usize) {
    RUNTIME_POPULATION_SIZE.with(|c| c.set(size));
}

// =====================
// Evolution / Population
// =====================
/// Number of agents/genomes in the population and per episode
pub const POPULATION_SIZE: usize = 30;
/// Episodes per generation for fitness averaging
pub const EPISODES_PER_GEN: usize = 3;

// ======
// World
// ======
pub const WORLD_W: f32 = 750.0;
pub const WORLD_H: f32 = 750.0;
// Agent starting and maximum energy (these are default values; use get_* functions for runtime values)
pub const INITIAL_ENERGY: f32 = 500.0;
pub const MAX_ENERGY: f32 = 5000.0;  // clamp upper bound for energy; can be >= INITIAL_ENERGY
pub const ENERGY_DRAIN_PER_STEP: f32 = 0.05;
pub const MAX_STEPS: usize = 20_000;
pub const AGENT_RADIUS: f32 = 1.5;

// =============
// Spawning
// =============
// When creating a new live episode, place agents in species-specific clusters
// to improve the odds of conspecific encounters (mating, grouping).
// Standard deviation of the Gaussian jitter around each species center (world units).
pub const SPECIES_SPAWN_CLUSTERING_ENABLED: bool = true;
pub const SPECIES_SPAWN_CLUSTER_STD: f32 = 30.0;
// Target number of members per cluster center; large species will get multiple centers.
pub const SPECIES_CLUSTER_TARGET_SIZE: usize = 10;
// Upper bound to avoid creating too many centers for very large species.
pub const SPECIES_SPAWN_MAX_CENTERS_PER_SPECIES: usize = 10;
// Disable clustering for very large populations to reduce Episode::new overhead
pub const SPECIES_CLUSTER_DISABLE_THRESHOLD: usize = 200;

// =====
// Food
// =====

// =====================
// Biomes (Phase 5)
// =====================
// We split the world into 3 vertical biomes by X coordinate with different growth rates.
// Define splits in normalized [0,1] of WORLD_W; with 2 splits we get 3 biomes: [0, s0), [s0, s1), [s1, 1]
pub const BIOME_X_SPLITS: [f32; 2] = [0.33, 0.66];
// Multipliers applied to base FOOD_RESPAWN_PROB and FOOD_SPREAD_CHANCE in each biome
pub const BIOME_RESPAWN_MULT: [f32; 3] = [0.6, 1.0, 1.6];
pub const BIOME_SPREAD_MULT: [f32; 3] = [0.5, 1.0, 1.8];
// Optional seasonal modulation: per-biome phase offsets to nudge migration/exploration
pub const SEASONAL_ENABLED: bool = true;
pub const SEASONAL_PERIOD_STEPS: usize = 4000; // higher = slower seasons
pub const SEASONAL_AMPLITUDE: f32 = 0.35;      // 0.0..1.0; multiplies growth by (1 + A*sin(...))
pub const BIOME_SEASON_PHASE: [f32; 3] = [0.0, 1.2, 2.4]; // radians offset per biome
pub const FOOD_COUNT: usize = 300;
pub const FOOD_RADIUS: f32 = 1.2;
pub const FOOD_ENERGY: f32 = 60.0;

// Plant/food dynamics
pub const MAX_FOOD: usize = 100;
pub const FOOD_MIN_SEP: f32 = 2.5;
pub const FOOD_RESPAWN_PROB: f32 = 0.006;
pub const FOOD_SPREAD_CHANCE: f32 = 0.01;
pub const FOOD_SPREAD_RADIUS: f32 = 15.0;

// =====================
// Vision cone parameters
// =====================
pub const VISION_RAYS: usize = 10;
pub const VISION_ANGLE_DEG: f32 = 90.0;  // Narrower cone (was 80.0)
pub const VISION_RANGE: f32 = 100.0;     // Longer range (was 90.0)
/// Derived: radians for convenience if needed by math

// ========
// Movement
// ========
pub const MAX_SPEED: f32 = 2.5; // legacy MAX_TURN & coupling removed (angle+speed model)

// ==============
// Sensing / Memory
// ==============
pub const DANGER_VECTOR_MAX_RANGE: f32 = 100.0; // used for danger memory attenuation
// Memory decay factor applied each step AFTER new memories are written (closer to 1.0 = slower decay)
pub const MEMORY_DECAY: f32 = 0.90;

// Inputs layout (revised vision; density removed):
//  1. Vision distances (nearest per sector/category): sectors (L,F,R)=3 × categories (Plant, Carcass, Same, Other, Wall)=5 => 15
//     Value encoding: normalized distance d/VISION_RANGE (0 near .. 1 far/no target). If no target in sector, value = 1.
//  2. Energy scalar = 1
//  3. Memory vectors (food_x, food_y, same_x, same_y, other_x, other_y) = 6
//  4. Hearing sectors (L,F,R) smoothed call intensity = HEARING_SECTORS (3)
//  5. Normalized absolute position (x/WORLD_W, y/WORLD_H) = 2
// Total INPUTS = 15 + 1 + 6 + HEARING_SECTORS + 2
// Temporarily disable hearing inputs entirely
pub const HEARING_SECTORS: usize = 0;
pub const INPUTS: usize = 15 + 1 + 6 + HEARING_SECTORS + 2; // now 24 total when HEARING_SECTORS=0
// Movement controller outputs now: [ turn, speed ] (relative turn model)
// turn in [-1,1] -> applied delta heading in [-MAX_TURN_PER_STEP, MAX_TURN_PER_STEP]
// speed in [-1,1] -> [0,1]
// Outputs: when communication is enabled => [ turn, speed, call ]; otherwise => [ turn, speed ]
// call in [-1,1] mapped to [0,1] intensity broadcast this step (available to others next step)
// We derive OUTPUTS from the COMMUNICATION_ENABLED flag so UI and initial genomes match the runtime mode.
pub const OUTPUTS: usize = 2 + (COMMUNICATION_ENABLED as usize);

// ==============================
// Input modality enable flags (compile-time)
// Set these before running to include (true) or mask out (false) a modality.
// Masking zeroes that segment of the input vector but keeps layout/length stable.
pub const ENABLE_VISION_INPUTS: bool = true;   // pooled sector proximities (plant/same/other/wall)
pub const ENABLE_HEARING_INPUTS: bool = false;  // heard call energy sectors
pub const ENABLE_MEMORY_INPUTS: bool = true;   // last food (x,y), same (x,y), other (x,y) memory vectors (6 floats)
// Density inputs removed in revised vision model

// ==========================
// Exploration (simplified)
// ==========================
pub const EXPL_CELL_SIZE: f32 = 50.0;            // grid resolution for exploration coverage
pub const EXPL_WEIGHT: f32 = 30.0;                // reward for 100% coverage (typically unreachable)

// ========================
// Core movement & fitness (simplified)
// ========================
// Fitness: we collapse plant/meat shaping into two simple weights.
pub const PLANT_FITNESS: f32 = 6.0;            // reward per plant eaten
pub const MEAT_FITNESS: f32 = 10.0;             // reward per meat (kill or scavenged corpse) event
pub const SURVIVAL_STEP_FITNESS: f32 = 0.00;  // reward per simulation step survived (alive or not? counted via total steps for now)
// Updated: SURVIVAL_STEP_FITNESS now applied per-agent using alive_steps^SURVIVAL_TIME_EXP
pub const SURVIVAL_TIME_EXP: f32 = 0.75;       // 0.5 => sqrt diminishing returns; 1.0 would be linear
// Predation shaping: reward successful attack hits and kills caused
pub const ATTACK_HIT_FITNESS: f32 = 2.5;       // reward per successful damage application to a live target
pub const KILL_CAUSED_FITNESS: f32 = 12.0;      // reward when an agent's hit reduces target health to <= 0
// Energy shaping: reward agents that maintain higher average energy while alive
// avg_energy is normalized by MAX_ENERGY before weighting
pub const ENERGY_AVG_WEIGHT: f32 = 10.0;
// Reproduction shaping: reward per successful birth (applied to each parent)
// This is applied during ECO culling by adding offspring_count * REPRO_BIRTH_FITNESS_PARENT
// to the parent's score. Keep modest to avoid runaway reproduction loops.
pub const REPRO_BIRTH_FITNESS_PARENT: f32 = 8.0;

// Social/Herding shaping: small per-step reward when staying behind/near same-species
// We use the same-species memory vector in local coordinates (x=right, y=forward) and
// reward positive forward alignment (same_y > 0). This nudges agents to follow others
// in front of them without forcing tight clustering. Keep this small.
pub const HERDING_ENABLED: bool = true;
pub const HERDING_REWARD_PER_STEP: f32 = 0.01; // applied as HERDING_REWARD_PER_STEP * max(0, same_y)
// Intake penalty: penalize agents with very low or zero intake to discourage camping/aimless wandering
// If an agent eats fewer than INTAKE_MIN_EVENTS times, apply a linear penalty per missing event.
// Example: INTAKE_MIN_EVENTS=2, INTAKE_MISS_PENALTY=5.0 => 0 eats: -10, 1 eat: -5, 2+ eats: 0
pub const INTAKE_MIN_EVENTS: usize = 10;
pub const INTAKE_MISS_PENALTY: f32 = 5.0;
// Communication economics
pub const CALL_COST: f32 = 0.003;             // linear energy cost per step scaled by call_intensity (0..1)
// Master switch to enable/disable communication features (calls, signals, hearing effects)
pub const COMMUNICATION_ENABLED: bool = false; // set to true to enable; false turns off calls completely
// Communication shaping (optional; set rewards small to avoid overpowering core objectives)
pub const COMM_SIGNAL_THRESHOLD: f32 = 0.40;   // minimum call_intensity to register a resource signal
pub const COMM_SIGNAL_WINDOW: usize = 40;      // steps a signal remains active
pub const COMM_FOOD_RADIUS: f32 = 25.0;        // within this distance of caller to consider signal relevant to resource
pub const COMM_FOOD_MIN: usize = 2;            // minimum food items in radius to mark signal as a valid resource broadcast
pub const COMM_SIGNAL_EFFECT_RADIUS: f32 = 60.0; // receivers must eat within this distance of original signal position
pub const COMM_RECV_REWARD: f32 = 0.8;         // fitness added to eater when benefiting from a signal
pub const COMM_CALLER_REWARD: f32 = 0.4;       // fitness added to original caller (smaller encourages some altruism)
// Rationale: focus on emergent behavior; keep only outcome-based signals (resource intake, exploration, longevity).

// If enabled, the example runs continuously without generational replacement.
// Agents may reproduce during an episode to spawn mutated offspring; dead/consumed
// agents are removed. The genomes vector is kept aligned with the agents vector.
pub const ECO_CONTINUOUS: bool = true;
// Hard caps and thresholds
pub const ECO_MAX_POP: usize = 250;                 // maximum concurrent agents
pub const ECO_MIN_POP: usize = 10;                 // minimum seeding on reset if all die
pub const ECO_BIRTH_ENERGY_THRESHOLD: f32 = 250.0; // minimum energy to allow birth
pub const ECO_BIRTH_ENERGY_COST: f32 = 50.0;      // energy deducted from parent per birth
pub const ECO_BIRTH_COOLDOWN_STEPS: usize = 40;    // steps before the same parent can reproduce again
pub const ECO_MAX_OFFSPRING_PER_AGENT: usize = 50;  // per-episode cap
pub const ECO_NEWBORN_ENERGY: f32 = 150.0;         // initial energy for newborns
pub const ECO_NEWBORN_HEALTH: f32 = AGENT_BASE_HEALTH / 2.0;
/// Distance within which two same-species, eligible parents can mate to produce an offspring
pub const ECO_MATE_RADIUS: f32 = 8.0 * AGENT_RADIUS;
// Batched speciation (eco continuous): run full speciation every N simulation steps OR when
// pending births exceed a threshold, instead of per-birth, to reduce pauses at high population.
pub const RESPEC_INTERVAL_STEPS: usize = 25; // tune: bigger = fewer speciation passes
pub const RESPEC_MAX_PENDING: usize = 40;    // trigger early if many births accumulate

// Newborn grace removed: no temporary invulnerability; newborns behave as adults immediately.
// Visual: how long to show a bright birth flash ring after an agent is created
pub const NEWBORN_FLASH_STEPS: usize = 18;

// =====================
// Predation/scavenging
// =====================
pub const EAT_AGENT_RADIUS: f32 = 2.5 * AGENT_RADIUS;
pub const MEAT_ENERGY: f32 = 220.0;
pub const PREDATION_ENABLED: bool = true;
pub const SCAVENGE_ENABLED: bool = true;
/// Require live prey to be within predator's vision cone to attack (enables ambush tactics)
pub const PREDATION_REQUIRES_VISION: bool = true;
/// Require corpses to be within vision cone to scavenge (set false for easier scavenging)
pub const SCAVENGE_REQUIRES_VISION: bool = false;
// Health / injury system
pub const AGENT_BASE_HEALTH: f32 = 100.0;        // starting and max health baseline
pub const HEALTH_DECAY_PER_STEP: f32 = 0.0;      // passive health decay (0 to disable)
pub const INJURY_HEAL_RATE: f32 = 0.04;          // health regained per step while alive (scaled by energy fraction)
pub const EAT_HEAL_FRACTION: f32 = 0.10;         // fraction of max health restored on plant eat
pub const MEAT_HEAL_BONUS: f32 = 12.0;           // flat bonus health on meat intake (before clamp)
pub const PREDATION_DAMAGE: f32 = 80.0;          // health damage dealt on a successful predation attempt
pub const SCAVENGE_TOUCH_DAMAGE: f32 = 0.0;      // health damage to scavenger when consuming corpse (risk factor)
pub const INVULN_AFTER_HIT_STEPS: usize = 2;     // brief invulnerability frames after taking damage
pub const HEALTH_TO_ENERGY_RATIO: f32 = 0.25;    // when health reaches 0 convert leftover health deficit to energy penalty (soft coupling)
pub const DEATH_HEALTH_THRESHOLD: f32 = 0.0;     // health <= this means agent dead (corpse logic kicks in)

// ===============================
// Idleness Penalty
// ===============================
/// Enable penalty for staying in the same place too long
pub const IDLENESS_PENALTY_ENABLED: bool = true;
/// Number of steps before idleness penalty kicks in (5 seconds ≈ varies by sim speed, using steps)
pub const IDLENESS_THRESHOLD_STEPS: usize = 100;
/// Distance threshold to consider agent as "staying in same place"
pub const IDLENESS_DISTANCE_THRESHOLD: f32 = 3.0;
/// Fitness penalty applied per step when idle beyond threshold
pub const IDLENESS_PENALTY_PER_STEP: f32 = 0.02;

// ===============================
// Motor model (relative turn + speed)
// ===================================
// Output[0] gives a turn command each step; we scale it by MAX_TURN_PER_STEP and add to heading.
// Output[1] gives speed scalar. Heading is wrapped to (-PI, PI] to avoid drift.
pub const MOTOR_NOISE: f32 = 0.0;                // noise disabled (was 0.03) for deterministic control
pub const MAX_TURN_PER_STEP: f32 = std::f32::consts::PI / 18.0; // same numeric value as previous smoothing limit
pub const MOVE_ENERGY_SCALE: f32 = 0.5;           // energy cost per unit normalized speed
pub const TURN_ENERGY_SCALE: f32 = 0.75;          // energy cost added proportional to |turn_fraction|
// Inertia extension (Stage A): treat Output[1] as forward thrust instead of direct speed.
// v_{t+1} = v_t * (1.0 - DRAG_COEFF) + thrust * MAX_THRUST * forward_dir
// Speed capped softly by MAX_VELOCITY (explicit clamp)
pub const USE_INERTIA: bool = true;               // feature flag to revert easily
pub const DRAG_COEFF: f32 = 0.10;                 // fraction of velocity lost per step (0.1 -> ~63% after 10 steps)
pub const MAX_THRUST: f32 = 1.0;                  // units of velocity added when thrust output = 1.0
pub const MAX_VELOCITY: f32 = 4.5;                // hard cap on velocity magnitude (pre world scaling)
pub const EXTRA_VEL_ENERGY_C1: f32 = 0.1;        // linear velocity cost term
pub const EXTRA_VEL_ENERGY_C2: f32 = 0.01;       // cubic velocity cost term (penalize high bursts)

// ==============
// Agent Collisions
// ==============
// Enable simple physical separation when agents overlap (alive vs alive).
pub const AGENT_COLLISIONS_ENABLED: bool = true;
// Number of relaxation passes per tick (1 is usually enough for POPULATION_SIZE ~ 50)
pub const AGENT_COLLISION_PASSES: usize = 2;

// ==============================
// Sensing smoothing
// ==============================
// (Removed) POOL_EMA_ALPHA – pooled proximities removed in revised vision model
// Hearing smoothing (separate in case we want different responsiveness)
pub const HEARING_EMA_ALPHA: f32 = 0.5;
// Max range for hearing (sound propagation)
pub const SOUND_RANGE: f32 = 160.0;
// Exponent for distance attenuation weight = (1 - d/R)^EXP (then squared by intensity, see implementation)
pub const SOUND_ATTENUATION_EXP: f32 = 2.0;

// ==============================
// Digestion / Corpse decay
// ==============================
pub const CORPSE_INITIAL_ENERGY: f32 = MEAT_ENERGY;
pub const CORPSE_DECAY_RATE: f32 = 0.002;
pub const DIGEST_STEPS_PLANT: u16 = 25;
pub const DIGEST_STEPS_MEAT: u16 = 45;

// =============================
// Speciation (visualizer)
// =============================
// Target number of species and adaptation rate for the compatibility threshold.
pub const SPECIES_TARGET: usize = 8;     // e.g., aim for ~8 species
pub const SPECIES_ADAPT_RATE: f32 = 0.01; // how fast the threshold adapts towards target
// During eco culling, keep at least this many per species (subject to POPULATION_SIZE cap)
pub const ECO_CULL_MIN_PER_SPECIES: usize = 10;

// Optional: Equal allocation among top-K species during ECO culling.
// When enabled, we rank species by their best member's fitness in the just-finished episode
// and keep an equal number of individuals from each of the top-K species. The quota is
// POPULATION_SIZE / K with the remainder distributed one-by-one starting from the best species.
// If any selected species has fewer available members than its quota, we take all it has and
// then fill any remaining population slots by global fitness order (across all remaining
// individuals regardless of species). When disabled, we fall back to the per-species minimum
// (ECO_CULL_MIN_PER_SPECIES) plus global fill policy.
pub const EQUAL_ALLOC_ENABLED: bool = true;
pub const EQUAL_ALLOC_TOP_K: usize = 6; // effective K is min(this, number of species)
// =============================
// Snapshotting
// =============================
// If > 0, automatically save a population snapshot every N generations.
pub const SNAPSHOT_INTERVAL: usize = 0; // set to e.g. 500 to enable periodic saves
