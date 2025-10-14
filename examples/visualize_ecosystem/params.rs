// Centralized simulation parameters
//
// This file groups knobs by domain: world, plants, perception, movement,
// ecology, sensing/memory, fitness/shaping, digestion, evolution/speciation.
// Where possible, keep names stable to avoid touching call sites.

// =====================
// Evolution / Population
// =====================
/// Number of agents/genomes in the population and per episode
pub const POPULATION_SIZE: usize = 50;
/// Episodes per generation for fitness averaging
pub const EPISODES_PER_GEN: usize = 3;

// ======
// World
// ======
pub const WORLD_W: f32 = 500.0;
pub const WORLD_H: f32 = 500.0;
// Agent starting and maximum energy
pub const INITIAL_ENERGY: f32 = 250.0;
pub const MAX_ENERGY: f32 = 1000.0;  // clamp upper bound for energy; can be >= INITIAL_ENERGY
pub const ENERGY_DRAIN_PER_STEP: f32 = 0.25;
pub const MAX_STEPS: usize = 500;
pub const AGENT_RADIUS: f32 = 1.5;

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
pub const FOOD_COUNT: usize = 100;
pub const FOOD_RADIUS: f32 = 1.2;
pub const FOOD_ENERGY: f32 = 60.0;

// Plant/food dynamics
pub const MAX_FOOD: usize = 100;
pub const FOOD_MIN_SEP: f32 = 2.5;
pub const FOOD_RESPAWN_PROB: f32 = 0.01;
pub const FOOD_SPREAD_CHANCE: f32 = 0.01;
pub const FOOD_SPREAD_RADIUS: f32 = 15.0;

// =====================
// Vision cone parameters
// =====================
pub const VISION_RAYS: usize = 7;
pub const VISION_ANGLE_DEG: f32 = 70.0;
pub const VISION_RANGE: f32 = 50.0;
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
//  3. Satiety scalar = 1 (normalized hunger level in [0,1]; 0=starving, 1=full)
//  4. Memory vectors (food_x, food_y, danger_x, danger_y) = 4
//  5. Hearing sectors (L,F,R) smoothed call intensity = HEARING_SECTORS (3)
//  6. Normalized absolute position (x/WORLD_W, y/WORLD_H) = 2
// Total INPUTS = 15 + 1 + 1 + 4 + HEARING_SECTORS + 2
// Temporarily disable hearing inputs entirely
pub const HEARING_SECTORS: usize = 0;
pub const INPUTS: usize = 15 + 1 + 1 + 4 + HEARING_SECTORS + 2; // now 23 total
// Movement controller outputs now: [ turn, speed ] (relative turn model)
// turn in [-1,1] -> applied delta heading in [-MAX_TURN_PER_STEP, MAX_TURN_PER_STEP]
// speed in [-1,1] -> [0,1]
// Outputs: when communication is enabled => [ turn, speed, call ]; otherwise => [ turn, speed ]
// call in [-1,1] mapped to [0,1] intensity broadcast this step (available to others next step)
// We derive OUTPUTS from the COMMUNICATION_ENABLED flag so UI and initial genomes match the runtime mode.
pub const OUTPUTS: usize = 2 + (COMMUNICATION_ENABLED as usize);

// =====================
// Hunger / Satiety
// =====================
// Agents have a normalized satiety value in [0,1]. It passively decays each step
// (hunger increases). Eating (plant or meat) increases satiety; digestion delivers
// energy over multiple steps and should also increase satiety as energy is received.
// Satiety can be used by behaviours or fitness shaping (not yet wired into inputs).
// Satiety parameters: slower passive decay and gentler per-step conversion so agents
// stay nourished longer but can still convert their satiety reserve into energy.
// Decrease `SATIETY_DECAY_PER_STEP` to make hunger build up more slowly.
// Further reduced for better learning: agents now have much more time to associate hunger with seeking food.
pub const SATIETY_DECAY_PER_STEP: f32 = 0.0001; // passive hunger increase per step (was 0.0004, now 4x slower!)
pub const SATIETY_GAIN_FROM_PLANT: f32 = 0.8; // immediate satiety gain when eating a plant (increased from 0.25)
pub const SATIETY_GAIN_FROM_MEAT: f32 = 1.2;   // immediate satiety bump when consuming meat (increased from 0.8)
// When digestion delivers an energy-equivalent, increase satiety proportionally.
pub const SATIETY_GAIN_PER_ENERGY_DELIVERED: f32 = 0.002; // per 1 energy delivered via digestion (was 0.0015)
// Convert satiety into usable energy each step. We consume up to SATIETY_CONSUME_PER_STEP
// of satiety and grant ENERGY_PER_SATIETY energy per 1.0 satiety consumed.
// Reduce the amount of satiety consumed per step so the reserve drains slower,
// and increase `ENERGY_PER_SATIETY` so the agent can still meet maintenance costs.
// Current: 0.005 × 200.0 = 1.0 energy/step (4x base drain, allows movement budget)
pub const SATIETY_CONSUME_PER_STEP: f32 = 0.005; // satiety consumed per step for energy conversion
pub const ENERGY_PER_SATIETY: f32 = 110.0; // energy gained per 1.0 satiety consumed (high to support active agents)

// ==============================
// Input modality enable flags (compile-time)
// Set these before running to include (true) or mask out (false) a modality.
// Masking zeroes that segment of the input vector but keeps layout/length stable.
pub const ENABLE_VISION_INPUTS: bool = true;   // pooled sector proximities (plant/same/other/wall)
pub const ENABLE_HEARING_INPUTS: bool = false;  // heard call energy sectors
pub const ENABLE_MEMORY_INPUTS: bool = true;   // last food & danger memory vectors (4 floats)
// Density inputs removed in revised vision model

// ==========================
// Exploration (simplified)
// ==========================
pub const EXPL_CELL_SIZE: f32 = 25.0;            // grid resolution for exploration coverage
pub const EXPL_WEIGHT: f32 = 10.0;                // reward for 100% coverage (typically unreachable)

// ========================
// Core movement & fitness (simplified)
// ========================
// Fitness: balanced food chain with dietary specialization
// Plants: abundant but low calorie (realistic herbivore niche)
// Meat: rare but high calorie (realistic carnivore niche)
pub const PLANT_FITNESS: f32 = 7.0;            // base reward per plant eaten (lower due to abundance)
pub const MEAT_FITNESS: f32 = 10.0;            // base reward per meat event (higher due to scarcity)

// Dietary specialization system: agents develop digestive efficiency based on eating history
// Diet ratio = meat_eaten / (plant_eaten + meat_eaten)
// - Herbivores (ratio < 0.3): efficient at plants, poor at meat digestion
// - Carnivores (ratio > 0.7): efficient at meat, poor at plant digestion  
// - Omnivores (0.3 <= ratio <= 0.7): balanced, can digest both moderately well
pub const DIET_HERBIVORE_THRESHOLD: f32 = 0.3;   // below this meat ratio = herbivore
pub const DIET_CARNIVORE_THRESHOLD: f32 = 0.7;   // above this meat ratio = carnivore
pub const DIET_SPECIALIST_BONUS: f32 = 1.5;      // multiplier for "correct" food (herbivore eating plants, carnivore eating meat)
pub const DIET_MISMATCH_PENALTY: f32 = 0.3;      // multiplier for "wrong" food (herbivore eating meat, carnivore eating plants)
pub const DIET_OMNIVORE_EFFICIENCY: f32 = 1.0;   // omnivores get full value from both (balanced)
pub const SURVIVAL_STEP_FITNESS: f32 = 0.08;  // reward per simulation step survived (increased to prioritize survival)
// Updated: SURVIVAL_STEP_FITNESS now applied per-agent using alive_steps^SURVIVAL_TIME_EXP
pub const SURVIVAL_TIME_EXP: f32 = 1.0;       // 1.0 = linear survival reward (was 0.75 with diminishing returns)
// Intake penalty: penalize agents with very low or zero intake to discourage camping/aimless wandering
// If an agent eats fewer than INTAKE_MIN_EVENTS times, apply a linear penalty per missing event.
// Example: INTAKE_MIN_EVENTS=2, INTAKE_MISS_PENALTY=5.0 => 0 eats: -10, 1 eat: -5, 2+ eats: 0
pub const INTAKE_MIN_EVENTS: usize = 2;
pub const INTAKE_MISS_PENALTY: f32 = 5.0;
// Communication economics
pub const CALL_COST: f32 = 0.003;             // linear energy cost per step scaled by call_intensity (0..1)
// Master switch to enable/disable communication features (calls, signals, hearing effects)
pub const COMMUNICATION_ENABLED: bool = false; // set to true to enable; false turns off calls completely
// Communication shaping (optional; set rewards small to avoid overpowering core objectives)
pub const COMM_SIGNAL_THRESHOLD: f32 = 0.40;   // minimum call_intensity to register a resource signal
pub const COMM_SIGNAL_WINDOW: usize = 40;      // steps a signal remains active
pub const COMM_FOOD_RADIUS: f32 = 25.0;        // within this distance of caller to consider signal relevant to resource
pub const COMM_FOOD_MIN: usize = 3;            // minimum food items in radius to mark signal as a valid resource broadcast
pub const COMM_SIGNAL_EFFECT_RADIUS: f32 = 60.0; // receivers must eat within this distance of original signal position
pub const COMM_RECV_REWARD: f32 = 0.8;         // fitness added to eater when benefiting from a signal
pub const COMM_CALLER_REWARD: f32 = 0.4;       // fitness added to original caller (smaller encourages some altruism)
// Rationale: focus on emergent behavior; keep only outcome-based signals (resource intake, exploration, longevity).

// If enabled, the example runs continuously without generational replacement.
// Agents may reproduce during an episode to spawn mutated offspring; dead/consumed
// agents are removed. The genomes vector is kept aligned with the agents vector.
pub const ECO_CONTINUOUS: bool = true;
// Hard caps and thresholds
pub const ECO_MAX_POP: usize = 80;                 // maximum concurrent agents
pub const ECO_MIN_POP: usize = 10;                 // minimum seeding on reset if all die
pub const ECO_BIRTH_ENERGY_THRESHOLD: f32 = 150.0; // minimum energy to allow birth
pub const ECO_BIRTH_ENERGY_COST: f32 = 50.0;      // energy deducted from parent per birth
pub const ECO_BIRTH_COOLDOWN_STEPS: usize = 80;    // steps before the same parent can reproduce again
pub const ECO_MAX_OFFSPRING_PER_AGENT: usize = 15;  // per-episode cap
pub const ECO_NEWBORN_ENERGY: f32 = 220.0;         // initial energy for newborns
pub const ECO_NEWBORN_HEALTH: f32 = AGENT_BASE_HEALTH;
/// Distance within which two same-species, eligible parents can mate to produce an offspring
pub const ECO_MATE_RADIUS: f32 = 8.0 * AGENT_RADIUS;

// =====================
// Predation/scavenging
// =====================
pub const EAT_AGENT_RADIUS: f32 = AGENT_RADIUS + AGENT_RADIUS;
pub const MEAT_ENERGY: f32 = 150.0;
pub const PREDATION_ENABLED: bool = true;
pub const SCAVENGE_ENABLED: bool = true;
// Health / injury system
pub const AGENT_BASE_HEALTH: f32 = 100.0;        // starting and max health baseline
pub const HEALTH_DECAY_PER_STEP: f32 = 0.0;      // passive health decay (0 to disable)
pub const INJURY_HEAL_RATE: f32 = 0.04;          // health regained per step while alive (scaled by energy fraction)
pub const EAT_HEAL_FRACTION: f32 = 0.10;         // fraction of max health restored on plant eat
pub const MEAT_HEAL_BONUS: f32 = 12.0;           // flat bonus health on meat intake (before clamp)
pub const PREDATION_DAMAGE: f32 = 55.0;          // base health damage dealt on a successful predation attempt
pub const HERBIVORE_DAMAGE_MULTIPLIER: f32 = 0.0; // herbivores deal 40% of base damage (weaker attackers)
pub const OMNIVORE_DAMAGE_MULTIPLIER: f32 = 0.8;  // omnivores deal 80% of base damage
pub const CARNIVORE_DAMAGE_MULTIPLIER: f32 = 1.2; // carnivores deal 120% of base damage (stronger attackers)
pub const SCAVENGE_TOUCH_DAMAGE: f32 = 0.0;      // health damage to scavenger when consuming corpse (risk factor)
pub const INVULN_AFTER_HIT_STEPS: usize = 6;     // brief invulnerability frames after taking damage
pub const HEALTH_TO_ENERGY_RATIO: f32 = 0.25;    // when health reaches 0 convert leftover health deficit to energy penalty (soft coupling)
pub const DEATH_HEALTH_THRESHOLD: f32 = 0.0;     // health <= this means agent dead (corpse logic kicks in)

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
pub const CORPSE_DECAY_RATE: f32 = 0.02;
pub const DIGEST_STEPS_PLANT: u16 = 25;
pub const DIGEST_STEPS_MEAT: u16 = 65;

// =============================
// Speciation (visualizer)
// =============================
// Target number of species and adaptation rate for the compatibility threshold.
pub const SPECIES_TARGET: usize = 8;     // e.g., aim for ~8 species
pub const SPECIES_ADAPT_RATE: f32 = 0.01; // how fast the threshold adapts towards target
// =============================
// Snapshotting
// =============================
// If > 0, automatically save a population snapshot every N generations.
pub const SNAPSHOT_INTERVAL: usize = 0; // set to e.g. 500 to enable periodic saves
