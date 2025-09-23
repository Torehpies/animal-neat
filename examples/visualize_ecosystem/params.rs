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
pub const WORLD_W: f32 = 1500.0;
pub const WORLD_H: f32 = 1500.0;
pub const INITIAL_ENERGY: f32 = 500.0;
pub const ENERGY_DRAIN_PER_STEP: f32 = 0.25;
pub const MAX_STEPS: usize = 500;
pub const AGENT_RADIUS: f32 = 1.5;

// =====
// Food
// =====
pub const FOOD_COUNT: usize = 150;
pub const FOOD_RADIUS: f32 = 1.2;
pub const FOOD_ENERGY: f32 = 60.0;

// Plant/food dynamics
pub const MAX_FOOD: usize = 250;
pub const FOOD_MIN_SEP: f32 = 2.5;
pub const FOOD_RESPAWN_PROB: f32 = 0.01;
pub const FOOD_SPREAD_CHANCE: f32 = 0.01;
pub const FOOD_SPREAD_RADIUS: f32 = 15.0;

// =====================
// Vision cone parameters
// =====================
pub const VISION_RAYS: usize = 5;
pub const VISION_ANGLE_DEG: f32 = 90.0;
pub const VISION_RANGE: f32 = 100.0;
/// Derived: radians for convenience if needed by math
pub const VISION_ANGLE_RAD: f32 = VISION_ANGLE_DEG.to_radians();

// ========
// Movement
// ========
pub const MAX_TURN: f32 = std::f32::consts::PI / 18.0;
pub const MAX_SPEED: f32 = 2.5;

// Reduce circling
pub const THRUST_TURN_COUPLING: f32 = 0.7;

// ==============
// Sensing / Memory
// ==============
pub const DANGER_VECTOR_MAX_RANGE: f32 = 150.0;
pub const DENSITY_SECTORS: usize = 8;
pub const DENSITY_RADIUS: f32 = 40.0;

// Inputs layout (ray-first, no direct food/meat vectors):
// - VISION_RAYS * 3 (per ray: food, wall, meat)
// - 1 (normalized energy)
// - 4 (memory vectors: last_food x,y and last_danger x,y)
// - DENSITY_SECTORS (alive-neighbor density bins)
pub const INPUTS: usize = VISION_RAYS * 3 + 1 + 4 + DENSITY_SECTORS;
// Outputs: [turn, thrust, sprint, brake]
pub const OUTPUTS: usize = 4;

// ==========================
// Exploration and avoidance
// ==========================
pub const EXPL_CELL_SIZE: f32 = 12.0;
// Exploration reward. Two modes:
// 1) Per-cell reward (legacy): EXPL_REWARD_PER_CELL per unique grid cell visited.
// 2) Normalized reward: fraction of world cells visited multiplied by EXPL_WEIGHT.
pub const EXPL_REWARD_PER_CELL: f32 = 0.06; // used when EXPL_NORMALIZE == false
pub const EXPL_NORMALIZE: bool = true;      // when true, use normalized exploration reward
pub const EXPL_WEIGHT: f32 = 10.0;          // total reward when covering 100% of the world (if EXPL_NORMALIZE)
pub const AVOID_RADIUS: f32 = 3.0;
pub const AVOID_PENALTY_SCALE: f32 = 0.003;
pub const AVOID_CHECK_EVERY: usize = 2;

// Legacy food-direction vector parameters (used for approach shaping)
pub const FOOD_VECTOR_MAX_RANGE: f32 = 150.0;

// ========================
// Turn cost and fitness shaping
// ========================
pub const TURN_COST: f32 = 0.02;
pub const EAT_WEIGHT: f32 = 4.5;
// Additional reward for meat-eating events (predation or scavenging)
pub const MEAT_WEIGHT: f32 = 4.0;
pub const STEP_WEIGHT: f32 = 0.001;
// Optional: sublinear gains for eaten count to reduce single-strategy domination.
// 1.0 keeps legacy linear behavior; 0.5 approximates sqrt.
pub const EAT_EXPONENT: f32 = 1.0;

// Headless-only shaping to reduce circling and radar scanning
// Reward getting closer to the nearest food, and penalize sustained turning.
pub const APPROACH_MAX_RANGE: f32 = FOOD_VECTOR_MAX_RANGE; // only count approach within this range
pub const APPROACH_REWARD_SCALE: f32 = 0.003;               // reward per unit distance improvement
pub const SPIN_PENALTY_SCALE: f32 = 0.001;                  // penalty per unit of |turn| per step

// =====================
// Predation/scavenging
// =====================
pub const EAT_AGENT_RADIUS: f32 = AGENT_RADIUS + AGENT_RADIUS;
pub const MEAT_ENERGY: f32 = 80.0;
pub const PREDATION_ENABLED: bool = true;
pub const SCAVENGE_ENABLED: bool = true;

// ===============================
// Motor model (sprint/brake)
// ===============================
// Sprint increases speed but adds extra energy cost; brake reduces speed with small cost.
pub const SPRINT_MULT: f32 = 1.6;
pub const BRAKE_MULT: f32 = 0.45;
pub const SPRINT_COST: f32 = 0.18;
pub const BRAKE_COST: f32 = 0.03;
pub const SPRINT_THRESHOLD: f32 = 0.5;
pub const BRAKE_THRESHOLD: f32 = 0.5;
// Small noise to motor outputs to improve robustness (uniform in [-NOISE, NOISE])
pub const MOTOR_NOISE: f32 = 0.05;

// ==============================
// Digestion / Corpse decay
// ==============================
pub const CORPSE_INITIAL_ENERGY: f32 = MEAT_ENERGY;
pub const CORPSE_DECAY_RATE: f32 = 0.08;
pub const DIGEST_STEPS_PLANT: u16 = 25;
pub const DIGEST_STEPS_MEAT: u16 = 65;

// =============================
// Speciation (visualizer)
// =============================
// Target number of species and adaptation rate for the compatibility threshold.
pub const SPECIES_TARGET: usize = 24;     // e.g., aim for ~8 species
pub const SPECIES_ADAPT_RATE: f32 = 0.1; // how fast the threshold adapts towards target
