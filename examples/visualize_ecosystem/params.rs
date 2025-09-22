// Centralized simulation parameters
pub const WORLD_W: f32 = 500.0;
pub const WORLD_H: f32 = 500.0;
pub const FOOD_COUNT: usize = 50;
pub const FOOD_RADIUS: f32 = 1.2;
pub const AGENT_RADIUS: f32 = 1.5;
pub const INITIAL_ENERGY: f32 = 300.0;
pub const ENERGY_DRAIN_PER_STEP: f32 = 0.4;
pub const FOOD_ENERGY: f32 = 65.0;
pub const MAX_STEPS: usize = 500;

// Plant/food dynamics
pub const MAX_FOOD: usize = 100;
pub const FOOD_MIN_SEP: f32 = 2.5;
pub const FOOD_RESPAWN_PROB: f32 = 0.06;
pub const FOOD_SPREAD_CHANCE: f32 = 0.02;
pub const FOOD_SPREAD_RADIUS: f32 = 15.0;

// Vision cone parameters
pub const VISION_RAYS: usize = 7;
pub const VISION_ANGLE_DEG: f32 = 90.0;
pub const VISION_RANGE: f32 = 200.0;

// Movement
pub const MAX_TURN: f32 = std::f32::consts::PI / 18.0;
pub const MAX_SPEED: f32 = 2.5;

// Reduce circling
pub const THRUST_TURN_COUPLING: f32 = 0.7;

// Predation/scavenging
pub const EAT_AGENT_RADIUS: f32 = AGENT_RADIUS + AGENT_RADIUS;
pub const MEAT_ENERGY: f32 = 90.0;
pub const PREDATION_ENABLED: bool = true;
pub const SCAVENGE_ENABLED: bool = true;

// Phase 3 sensing
pub const DANGER_VECTOR_MAX_RANGE: f32 = 150.0;
pub const DENSITY_SECTORS: usize = 8;
pub const DENSITY_RADIUS: f32 = 40.0;

pub const INPUTS: usize = VISION_RAYS * 2 + 3 + 4 + DENSITY_SECTORS;
// Outputs: [turn, thrust, sprint, brake]
pub const OUTPUTS: usize = 4;

// Exploration and avoidance
pub const EXPL_CELL_SIZE: f32 = 10.0;
pub const EXPL_REWARD_PER_CELL: f32 = 0.02;
pub const AVOID_RADIUS: f32 = 3.0;
pub const AVOID_PENALTY_SCALE: f32 = 0.003;
pub const AVOID_CHECK_EVERY: usize = 2;
pub const EPISODES_PER_GEN: usize = 3;

// Food-direction vector parameters
pub const FOOD_VECTOR_MAX_RANGE: f32 = 150.0;

// Turn and fitness shaping
pub const TURN_COST: f32 = 0.02;
pub const EAT_WEIGHT: f32 = 4.5;
pub const STEP_WEIGHT: f32 = 0.001;

// Headless-only shaping to reduce circling and radar scanning
// Reward getting closer to the nearest food, and penalize sustained turning.
pub const APPROACH_MAX_RANGE: f32 = FOOD_VECTOR_MAX_RANGE; // only count approach within this range
pub const APPROACH_REWARD_SCALE: f32 = 0.0;               // reward per unit distance improvement
pub const SPIN_PENALTY_SCALE: f32 = 0.0;                  // penalty per unit of |turn| per step

// Phase 4: motor model expansion
// Sprint increases speed but adds extra energy cost; brake reduces speed with small cost.
pub const SPRINT_MULT: f32 = 1.6;
pub const BRAKE_MULT: f32 = 0.45;
pub const SPRINT_COST: f32 = 0.18;
pub const BRAKE_COST: f32 = 0.03;
pub const SPRINT_THRESHOLD: f32 = 0.5;
pub const BRAKE_THRESHOLD: f32 = 0.5;
// Small noise to motor outputs to improve robustness (uniform in [-NOISE, NOISE])
pub const MOTOR_NOISE: f32 = 0.05;

// Phase 2: corpse decay and digestive lag
pub const CORPSE_INITIAL_ENERGY: f32 = MEAT_ENERGY;
pub const CORPSE_DECAY_RATE: f32 = 0.02;
pub const DIGEST_STEPS_PLANT: u16 = 25;
pub const DIGEST_STEPS_MEAT: u16 = 35;

// Speciation tuning (visualizer)
// Target number of species and adaptation rate for the compatibility threshold.
pub const SPECIES_TARGET: usize = 8;     // e.g., aim for ~8 species
pub const SPECIES_ADAPT_RATE: f32 = 0.05; // how fast the threshold adapts towards target
