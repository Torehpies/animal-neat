# neat

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
