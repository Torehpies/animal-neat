# Animal-NEAT Ecosystem Visualizer

A macroquad-based interactive demo that evolves simple agents using NEAT and visualizes their world, sensing, and behavior.

## Quick start

- Build & run the visualizer example:

```bash
# Release build is smoother
cargo run --release --example visualize_ecosystem
```

- Controls (visual-only):
  - P: pause/resume
  - F: fast mode on/off
  - R: restart episode (same generation)
  - V: toggle vision cones
  - U: toggle unified input overlay
  - E: toggle energy overlay
  - S: save population snapshot (snapshots/pop_snapshot_genXXXX.json)

## Configure experiment

Edit `examples/visualize_ecosystem/params.rs` to change world size, movement model, fitness weights, and modalities. To disable input modalities at compile-time (while keeping the input layout fixed):

```rust
pub const ENABLE_VISION_INPUTS: bool = true;
pub const ENABLE_HEARING_INPUTS: bool = true;
pub const ENABLE_MEMORY_INPUTS: bool = true;
pub const ENABLE_DENSITY_INPUTS: bool = true;
```

Snapshots can be enabled periodically via `SNAPSHOT_INTERVAL` or ad-hoc with the S key.

## Project structure

- `src/neat/*` — core NEAT library (genome, speciation, crossover, innovation tracker, IO)
- `examples/visualize_ecosystem/*` — the ecosystem demo
  - `params.rs` — centralized configuration and constants (single source of truth)
  - `sensing.rs` — builds neural inputs; pooling, density, hearing; `input_ranges()` helper
  - `sim.rs` — movement, predation/scavenging, corpse decay, digestion
  - `world.rs` — world state and plant lifecycle
  - `ui/` — rendering (world view, HUD, network)

## Inputs & outputs

Input vector layout (documented in `params.rs`):
- Vision pools: 3 sectors × 4 categories = 12
- Energy: 1
- Memory vectors (food_x, food_y, danger_x, danger_y): 4
- Density sectors: DENSITY_SECTORS
- Hearing sectors: HEARING_SECTORS
- Position (x/W, y/H): 2

Outputs: `[ turn, thrust, call ]`

## Fitness (simplified)
- Intake: plant and meat events with weights
- Exploration fraction over a coarse grid
- Survival with diminishing returns
- Optional communication rewards (caller/receiver) for resource signals

## Developing
- Prefer adding new constants to `params.rs` rather than scattering literals.
- Use `sensing::input_ranges()` to derive index ranges; avoid magic indices.
- Keep UI-only toggles separate from functional parameters.

## License
MIT (or your chosen license). Contributions welcome.# neat

A small, self-contained NEAT (NeuroEvolution of Augmenting Topologies) implementation in Rust.

## Features

- Minimal public API with sensible defaults
- JSON save/load for genomes via `serde`
- Example solving XOR

## Use as a library

Add to your Cargo.toml (path or git, as appropriate), then:

```rust
use neat::neat::runner; // or `use neat::runner;` thanks to re-exports

fn main() {
    runner::run_neat_xor(true, 200, 50, 3.9, 2.0);
}
```

## Run the included example

```powershell
# From the workspace root
cargo run --example quickstart
```

This will print generation logs and save the best genome to `best_genome.json` once the target fitness is reached.

## Ecosystem visualizer (real-time demo)

A live, multi-agent ecosystem demo with vision-cone sensing and NEAT-driven behavior is included.

```powershell
# Recommended: run in release mode for smooth rendering
cargo run --release --example visualize_ecosystem
```

Controls:
- P: pause/resume the live simulation
- F: toggle fast mode (advance multiple steps per frame)
- R: reset the episode (new world and agent positions)
- V: toggle drawing of the vision cones/rays
- D: toggle local density overlay
- B: toggle vector overlay (food/danger from rays)

Notes:
- Agents are colored by diet: green = herbivore, red = carnivore, pale before first meal.
- Ray cues: food hits are green; meat (live/scavenge) hits are orange; a short directional line appears when an edible is inside eat range.
- Parameters for vision, movement, fitness shaping, and speciation are centralized in the example’s params module.

## Binary

Running `cargo run` will execute a default XOR evolution run as well.

## Save/Load genomes

```rust
use neat::neat::{genome::Genome, innovation_tracker::InnovationTracker, io};

// ... after evolving and picking a genome `g`
io::save_genome_json("best_genome.json", &g)?;
let loaded: Genome = io::load_genome_json("best_genome.json")?;
```

## License

MIT
