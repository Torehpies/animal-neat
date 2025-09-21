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
