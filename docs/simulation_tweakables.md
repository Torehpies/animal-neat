# Simulation reproduction and tweakables

This file summarizes how reproduction works and the parameters you can tune to control the simulation.

## Reproduction overview (NEAT)

- Fitness assignment: Each genome’s `fitness` is set from the evaluation.
- Speciation: Genomes are grouped into species by compatibility distance. Species are sorted by best fitness.
- Survival:
  - The top species always survives to the next generation.
  - Other species survive if their `stagnant_generations` < `stagnation_limit` in `EvolutionConfig`.
- Elitism: Each surviving species contributes its single best genome directly to the next generation (no mutation).
- Offspring allocation:
  - The remaining slots are distributed to species proportional to their `adjusted_fitness` (average fitness of that species).
  - If rounding leaves the population short, the best surviving species is used to generate additional offspring.
- Offspring creation:
  - 1-member species: clone + mutate the sole member.
  - 2+ members: choose two parents from the species, crossover (fitter is dominant), then mutate.
- Population size: Stays constant at the previous generation’s size by design (no automatic growth/shrink).

## EvolutionConfig (genetic operators)

From `src/neat/config.rs`:

- `conn_mutation_rate` (f32): Probability to add a connection during mutation (e.g., 0.05)
- `node_mutation_rate` (f32): Probability to add a node (split a connection) during mutation (e.g., 0.03)
- `weight_mutation_rate` (f32): Probability to perturb a connection weight (e.g., 0.8)
- `bias_mutation_rate` (f32): Probability to perturb a node bias (e.g., 0.7)
- `weight_perturb_power` (f32): Magnitude of weight perturbations (e.g., 0.5)
- `bias_perturb_power` (f32): Magnitude of bias perturbations (e.g., 0.5)
- `reinit_weight_prob` (f32): Probability to reinitialize a weight instead of perturbing (e.g., 0.1)
- `reinit_bias_prob` (f32): Probability to reinitialize a bias instead of perturbing (e.g., 0.1)
- `stagnation_limit` (usize): Generations with no improvement before a species is considered stagnant (e.g., 15)
- `compatibility_threshold` (f32): Baseline threshold used for species compatibility distance

Note: The visualizer sets up `Speciator` with an adaptive threshold targeting ~5 species (configurable in code) so the effective threshold may move across generations.

## Visualizer and environment parameters

In `examples/visualize_ecosystem.rs` you can tune:

- World/agent
  - `WORLD_W`, `WORLD_H`: World dimensions
  - `AGENT_RADIUS`, `FOOD_RADIUS`
  - `INITIAL_ENERGY`, `ENERGY_DRAIN_PER_STEP`, `FOOD_ENERGY`
  - Movement: `MAX_TURN`, `MAX_SPEED`
  - Thrust-turn coupling: `THRUST_TURN_COUPLING` (0..1) — reduces thrust based on |turn| to avoid circling

- Vision
  - `VISION_RAYS`, `VISION_ANGLE_DEG`, `VISION_RANGE`
  - Inputs: per-ray [food, wall] + energy (last input)

- Plants (food) dynamics
  - `FOOD_COUNT`: initial plants, `MAX_FOOD`: cap on plants
  - `FOOD_MIN_SEP`: minimum separation between plants
  - `FOOD_RESPAWN_PROB`: per-step probability to spawn a random plant
  - `FOOD_SPREAD_CHANCE`: per-plant chance to spawn a nearby offshoot per step
  - `FOOD_SPREAD_RADIUS`: radius for offshoots

- Predation/scavenging
  - `PREDATION_ENABLED`, `SCAVENGE_ENABLED`
  - `EAT_AGENT_RADIUS`: eat distance for agents
  - `MEAT_ENERGY`: energy gained when eating an agent (live or dead)

- Fitness shaping (used in headless eval)
  - Exploration: `EXPL_CELL_SIZE`, `EXPL_REWARD_PER_CELL`
  - Avoidance: `AVOID_RADIUS`, `AVOID_PENALTY_SCALE`, `AVOID_CHECK_EVERY`
  - Episodes: `EPISODES_PER_GEN`

- Visualizer pacing
  - Normal mode step interval: `normal_step_interval`
  - Fast mode: `fast_steps_per_frame`

## Suggested live tuning ideas

If desired, add hotkeys to tweak a few parameters in runtime (and display them in HUD):
- Increase/decrease `FOOD_RESPAWN_PROB` and `FOOD_SPREAD_CHANCE`
- Toggle `PREDATION_ENABLED`, `SCAVENGE_ENABLED`
- Adjust `THRUST_TURN_COUPLING`

This helps explore dynamics without recompiling.
