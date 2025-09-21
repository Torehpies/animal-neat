# NEAT library usage guide

This guide shows how to use the NEAT library to run your own simulations.

- Quick start (XOR)
- Running a custom simulation (your own fitness)
- Saving and loading genomes
- Evaluating a trained genome
- Tuning knobs and tips

## Quick start (XOR)

The library includes a built-in runner for the XOR problem:

```rust
use neat::runner;

fn main() {
    runner::run_neat_xor(
        true,   // save_best: write best_genome.json when target achieved
        1000,   // generations
        50,     // population size
        3.9,    // target fitness (max is ~4.0 for XOR)
        2.0,    // speciator compatibility threshold (lower = more species)
    );
}
```

From this repo, you can also run the example:

```powershell
cargo run --example quickstart
```

## Concepts

- Genome: A neural network topology + weights (nodes + connections) with a `fitness` value.
- InnovationTracker: Assigns unique innovation numbers to new structural genes.
- Speciator: Groups genomes into species based on genetic distance for speciation.
- Evolution: Produces the next generation via selection, crossover, and mutation.
- Fitness function: Problem-specific scoring function mapping a genome to a float (higher is better).

## Running a custom simulation

Here’s a minimal pattern to evolve on your own dataset/fitness function.

```rust
use neat::neat::{
    evolution,
    genome::Genome,
    innovation_tracker::InnovationTracker,
    speciator::Speciator,
};

fn my_fitness(g: &Genome) -> f32 {
    // Example: XOR (replace with your problem)
    let x_data = vec![vec![0.0, 0.0], vec![0.0, 1.0], vec![1.0, 0.0], vec![1.0, 1.0]];
    let y_data = vec![0.0, 1.0, 1.0, 0.0];

    let mut total_err = 0.0;
    for (x, y) in x_data.into_iter().zip(y_data) {
        let out = g.evaluate(x);
        let val = out.get(0).copied().unwrap_or(0.0);
        total_err += (val - y).abs();
    }
    (4.0 - total_err).max(0.0) // higher is better
}

fn main() {
    let num_inputs = 2;
    let num_outputs = 1;
    let pop_size = 50;
    let generations = 200;
    let target_fitness = 3.9;

    let mut innov = InnovationTracker::new();
    let mut speciator = Speciator::new(2.0);

    // 1) Initialize population
    let mut population = Genome::create_initial_population(pop_size, num_inputs, num_outputs, &mut innov);

    for gen in 0..generations {
        // 2) Score fitness
        let fitness_scores: Vec<f32> = population.iter().map(my_fitness).collect();

        // 3) Inspect progress
        let best = fitness_scores
            .iter()
            .cloned()
            .fold(f32::NEG_INFINITY, f32::max);
        let avg = fitness_scores.iter().sum::<f32>() / fitness_scores.len() as f32;
        println!("Gen {gen}: best={best:.4}, avg={avg:.4}");

        // Early stop
        if best >= target_fitness {
            println!("Target reached at generation {gen}");
            break;
        }

        // 4) Evolve next generation
        population = evolution::evolution(
            population,
            fitness_scores,
            &mut speciator,
            &mut innov,
            15, // stagnation limit in generations per-species
        );
    }
}
```

There’s a complete version as an example: `examples/custom.rs`.

Run it with:

```powershell
cargo run --example custom
```

### Notes
- Mutation rates are currently set inside `evolution::reproduce_species`. You can tweak those in the library for different behaviors.
- Speciation sensitivity is controlled by the threshold passed to `Speciator::new(threshold)`; lower threshold tends to create more species.

## Saving and loading genomes

Use the JSON helpers:

```rust
use neat::io;
use neat::genome::Genome;

// After evolving
io::save_genome_json("best_genome.json", &best_genome)?;
let loaded: Genome = io::load_genome_json("best_genome.json")?;
```

## Evaluating a trained genome

Call `evaluate` with a vector of input floats. The length must match the number of input nodes used to initialize the genome/population.

```rust
let outputs = loaded.evaluate(vec![0.0, 1.0]);
println!("output = {:?}", outputs);
```

## Tuning knobs and tips

- Population size: Larger can improve exploration but takes longer (try 100–300).
- Generations: Give enough time to explore; early stopping helps.
- Speciation threshold: Lower → more species; adjust to keep diversity.
- Mutation strengths: Inside `Genome::mutate_*` you can adjust power/clamps.
- Fitness scaling: Ensure your fitness function ranges are sensible (non-negative, higher is better).

## Troubleshooting

- If a genome panics during `evaluate`, verify your input length matches your declared input node count.
- If you see stagnation, lower the speciation threshold or increase mutation rates.
- For deterministic tests, consider seeding `rand` when available (this simplified impl uses the default RNG).
