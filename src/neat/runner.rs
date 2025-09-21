use std::f32;

use crate::neat::{
    config::EvolutionConfig, evolution, fitness::fitness_xor, genome::Genome,
    innovation_tracker::InnovationTracker, io, speciator::Speciator,
};

/// Run a NEAT evolution process on the XOR problem.
///
/// Parameters:
/// - `save_best`: If true, writes the best genome to `best_genome.json` when the target is reached.
/// - `generations`: Maximum number of generations to run.
/// - `pop_size`: Number of genomes per generation.
/// - `target_fitness`: Early-stop threshold. For XOR, 4.0 is a perfect score.
/// - `speciator_threshold`: Compatibility threshold controlling species formation (lower → more species).
///
/// This function prints progress per generation and returns when the target is met or the generation
/// limit is reached.
pub fn run_neat_xor(
    save_best: bool,
    generations: usize,
    pop_size: usize,
    target_fitness: f32,
    speciator_threshold: f32,
) {
    let num_inputs = 2;
    let num_outputs = 1;

    let mut innov = InnovationTracker::new();
    let mut speciator = Speciator::new(speciator_threshold);
    let mut cfg = EvolutionConfig::default();
    cfg.compatibility_threshold = speciator_threshold;

    let mut population =
        Genome::create_initial_population(pop_size, num_inputs, num_outputs, &mut innov);

    let mut best_fitness_history: Vec<f32> = Vec::new();
    let mut avg_fitness_history: Vec<f32> = Vec::new();

    for generation in 0..generations {
        let fitness_scores: Vec<f32> = population.iter().map(fitness_xor).collect();

        //println!("fitness: {:?}", fitness_scores);

        let best_fitness = fitness_scores
            .iter()
            .cloned()
            .fold(f32::NEG_INFINITY, f32::max);
        let avg_fitness = fitness_scores.iter().sum::<f32>() / fitness_scores.len() as f32;

        best_fitness_history.push(best_fitness);
        avg_fitness_history.push(avg_fitness);

        println!(
            "Generation {}: Best={}, Avg={}",
            generation, best_fitness, avg_fitness
        );

        if best_fitness >= target_fitness {
            println!("Problem was solved in {} generations", generation);
            println!(
                "Best fitness achieved: {}",
                best_fitness_history
                    .iter()
                    .cloned()
                    .fold(f32::NEG_INFINITY, f32::max)
            );

            let best_index = fitness_scores
                .iter()
                .enumerate()
                .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
                .map(|(i, _)| i)
                .unwrap();

            let best_genome = &population[best_index];

            if save_best {
                if let Err(e) = io::save_genome_json("best_genome.json", best_genome) {
                    println!("Failed to save best genome: {}", e);
                } else {
                    println!("Saved best genome to best_genome.json");
                }
            }

            return;
        }

        // Evolve the population for the next generation
        population = evolution::evolution(population, fitness_scores, &mut speciator, &mut innov, &cfg);
    }
}
