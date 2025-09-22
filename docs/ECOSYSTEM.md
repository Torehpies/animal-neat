# Ecosystem (Multi‑Agent) — Vision Cones

This example simulates many agents in a grid world competing to eat food before running out of energy. Each genome controls one agent. Agents perceive the world through a forward‑facing vision cone and choose moves each step.

Key files:
- `examples/ecosystem.rs` — headless evolution loop (multi‑agent scoring in a shared world)
- `examples/visualize_ecosystem.rs` — real‑time visualization with controls

## World and Agents
- Grid: 32×32
- Food: 80 items spawned at random per episode
- Energy: starts at 100, drains 1 per step; eating food restores 20 (capped at 100)
- Steps: episode ends after 300 steps or when all agents are out of energy or all food is gone
- Agents: one per genome; each has position and a facing direction

## Inputs and Outputs
- Inputs: `VISION_RAYS * 2 + 1`
  - For each ray in the vision cone: two signals
    - food signal: 1.0 when a food is detected, decays with distance (0 if none in range)
    - wall signal: proximity to the nearest wall along that ray (0 if none within range)
  - plus agent energy normalized to `[0, 1]`
- Outputs (4): up, right, down, left — argmax determines the move and updates the agent’s facing

Vision parameters (edit in the example file):
- `VISION_RAYS` (default 5)
- `VISION_ANGLE_DEG` (default 90°)
- `VISION_RANGE` in cells (default 6)

## Fitness
For each agent:
- `fitness = eaten * 3.0 + steps * 0.01`

The population’s next generation is produced via NEAT using these per‑agent scores.

## Run
- Headless (evolve only):
  - Windows PowerShell
    - cargo run --example ecosystem
- Visualizer (macroquad window):
  - Windows PowerShell
    - cargo run --example visualize_ecosystem

## Visualizer Controls
- Space: step one generation
- A: toggle auto‑run (one generation every ~0.1s)
- R: reset the episode (new food + agent positions)
- V: toggle drawing of vision cones

HUD shows generation stats, best/avg fitness, and episode aggregates (total eaten, alive agents, avg energy, steps).

## Tuning
- Evolution parameters: adjust `EvolutionConfig` inside the example. Key knobs: mutation rates, compatibility threshold, stagnation limit.
- World parameters: tweak `GRID_W`, `GRID_H`, `FOOD_COUNT`, `INITIAL_ENERGY`, `ENERGY_DRAIN_PER_STEP`, `FOOD_ENERGY`, `MAX_STEPS`.
- Vision parameters: `VISION_RAYS`, `VISION_ANGLE_DEG`, `VISION_RANGE`.
- Population size: set when constructing `AppState::new(pop_size)` or in the headless example.

Tip: for faster learning, try more food, slightly longer episodes, and moderate mutation rates; switch to `--release` for speed.
