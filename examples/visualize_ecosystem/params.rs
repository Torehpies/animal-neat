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

// ========
// Movement
// ========
pub const MAX_SPEED: f32 = 2.5; // legacy MAX_TURN & coupling removed (angle+speed model)

// ==============
// Sensing / Memory
// ==============
pub const DANGER_VECTOR_MAX_RANGE: f32 = 150.0;
// Density sectors reduced: 5 custom bins (side-left, side-right, back-left, back-right, back-center)
// Rationale: compress spatial crowding signal while keeping coarse directional awareness.
pub const DENSITY_SECTORS: usize = 5;
pub const DENSITY_RADIUS: f32 = 40.0;

// Inputs layout (directional pooled proximities):
// Directional pooled proximities (updated): 3 angular sectors (Left, Forward, Right) each with 4 categories:
//   - PlantOrCarcass (static edible)
//   - SameAlive (alive conspecific)
//   - OtherAlive (alive heterospecific)
//   - Wall (boundary)
// Per sector features: 4 proximities -> 3 * 4 = 12.
// Remaining standard features: energy (1) + memories (4) + density sectors (5) = 10.
// Total INPUTS = 12 + 10 = 22.
pub const INPUTS: usize = 12 + 1 + 4 + DENSITY_SECTORS;
// Movement controller outputs now: [ turn, speed ] (relative turn model)
// turn in [-1,1] -> applied delta heading in [-MAX_TURN_PER_STEP, MAX_TURN_PER_STEP]
// speed in [-1,1] -> [0,1]
pub const OUTPUTS: usize = 2;

// ==========================
// Exploration (simplified)
// ==========================
// We always use normalized exploration: fraction of world cells visited * EXPL_WEIGHT.
// (Legacy per-cell / avoidance shaping removed to simplify fitness.)
pub const EXPL_CELL_SIZE: f32 = 12.0;            // grid resolution for exploration coverage
pub const EXPL_WEIGHT: f32 = 8.0;                // reward for 100% coverage (typically unreachable)

// Legacy food-direction vector parameters (used for approach shaping)
// (Removed FOOD_VECTOR_MAX_RANGE; nearest_food_vector_local unused)

// ========================
// Core movement & fitness (simplified)
// ========================
// Fitness: we collapse plant/meat shaping into two simple weights.
pub const PLANT_FITNESS: f32 = 4.0;            // reward per plant eaten
pub const MEAT_FITNESS: f32 = 4.0;             // reward per meat (kill or scavenged corpse) event
pub const SURVIVAL_STEP_FITNESS: f32 = 0.01;  // reward per simulation step survived (alive or not? counted via total steps for now)
// Removed: approach reward, spin penalty, crowding penalty, sublinear exponent.
// Rationale: focus on emergent behavior; keep only outcome-based signals (resource intake, exploration, longevity).

// =====================
// Predation/scavenging
// =====================
pub const EAT_AGENT_RADIUS: f32 = AGENT_RADIUS + AGENT_RADIUS;
pub const MEAT_ENERGY: f32 = 80.0;
pub const PREDATION_ENABLED: bool = true;
pub const SCAVENGE_ENABLED: bool = true;

// ===============================
// Motor model (relative turn + speed)
// ===================================
// Output[0] gives a turn command each step; we scale it by MAX_TURN_PER_STEP and add to heading.
// Output[1] gives speed scalar. Heading is wrapped to (-PI, PI] to avoid drift.
pub const MOTOR_NOISE: f32 = 0.03;               // small exploratory noise
pub const MAX_TURN_PER_STEP: f32 = std::f32::consts::PI / 18.0; // same numeric value as previous smoothing limit
pub const MOVE_ENERGY_SCALE: f32 = 0.2;           // energy cost per unit normalized speed
pub const TURN_ENERGY_SCALE: f32 = 0.01;          // energy cost added proportional to |turn_fraction|

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
