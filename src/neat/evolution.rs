use rand::seq::IndexedRandom;

use crate::neat::{
    crossover::crossover, genome::Genome, innovation_tracker::InnovationTracker,
    speciator::Speciator, species::Species,
};

pub fn reproduce_species(
    species: &Species,
    offspring_count: usize,
    innov: &mut InnovationTracker,
) -> Vec<Genome> {
    let conn_mutation_rate: f32 = 0.05;
    let node_mutation_rate: f32 = 0.03;
    let weight_mutation_rate: f32 = 0.8;
    let bias_mutation_rate: f32 = 0.7;

    let mut rng = rand::rng();
    let mut offspring = Vec::new();

    if species.members.len() == 1 {
        let parent = &species.members[0];
        for _ in 0..offspring_count {
            let mut child = Genome::new(
                parent.nodes.values().cloned().map(Some).collect(),
                parent.connections.clone(),
            );
            child.mutate(
                innov,
                conn_mutation_rate,
                node_mutation_rate,
                weight_mutation_rate,
                bias_mutation_rate,
            );
            offspring.push(child);
        }
    } else {
        for _ in 0..offspring_count {
            let parents: Vec<&Genome> = species.members.choose_multiple(&mut rng, 2).collect();
            let (mut parent1, mut parent2) = (parents[0], parents[1]);

            if parent1.fitness < parent2.fitness {
                std::mem::swap(&mut parent1, &mut parent2);
            }

            let mut child = crossover(parent1, parent2);
            child.mutate(
                innov,
                conn_mutation_rate,
                node_mutation_rate,
                weight_mutation_rate,
                bias_mutation_rate,
            );
            offspring.push(child);
        }
    }
    offspring
}

pub fn evolution(
    mut population: Vec<Genome>,
    fitness_scores: Vec<f32>,
    speciator: &mut Speciator,
    innov: &mut InnovationTracker,
    stagnation_limit: usize,
) -> Vec<Genome> {
    let mut new_population: Vec<Genome> = Vec::new();

    for (genome, &fitness) in population.iter_mut().zip(fitness_scores.iter()) {
        genome.fitness = fitness;
    }

    speciator.speciate(&population);
    let species_list = speciator.get_species_mut();
    species_list.sort_by(|a, b| b.best_fitness.partial_cmp(&a.best_fitness).unwrap());

    //println!("Species created: {}", species_list.len());

    let mut surviving_species: Vec<&mut Species> = Vec::new();

    if !species_list.is_empty() {
        let (first, rest) = species_list.split_at_mut(1);
        surviving_species.push(&mut first[0]);
        for s in rest {
            if s.stagnant_generations < stagnation_limit as u32 {
                surviving_species.push(s);
            }
        }
    }

    //println!("Species that survived: {}", surviving_species.len());

    let total_adjusted_fitness: f32 = surviving_species.iter().map(|s| s.adjusted_fitness).sum();
    //println!("Total adjusted fitness: {}", total_adjusted_fitness);

    for species in surviving_species.iter() {
        if !species.members.is_empty() {
            if let Some(best_genome) = species
                .members
                .iter()
                .cloned()
                .max_by(|a, b| a.fitness.total_cmp(&b.fitness))
            {
                new_population.push(best_genome);
            }
        }
    }

    let remaining_offspring = population.len() - new_population.len();

    for species in surviving_species.iter() {
        let offspring_count = if total_adjusted_fitness > 0.0 {
            ((species.adjusted_fitness / total_adjusted_fitness) * remaining_offspring as f32)
                .round() as usize
        } else {
            remaining_offspring / surviving_species.len().max(1)
        };

        if offspring_count > 0 {
            let offspring = reproduce_species(species, offspring_count, innov);
            new_population.extend(offspring);
        }
    }

    while new_population.len() < population.len() {
        if let Some(best_species) = surviving_species
            .iter()
            .max_by(|a, b| a.adjusted_fitness.total_cmp(&b.adjusted_fitness))
        {
            let offspring = reproduce_species(best_species, 1, innov);
            new_population.extend(offspring);
        }
    }

    new_population
}
