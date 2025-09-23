use rand::seq::IndexedRandom;

use crate::neat::{
    crossover::crossover, genome::Genome, innovation_tracker::InnovationTracker,
    speciator::Speciator, species::Species, config::EvolutionConfig,
};

pub fn reproduce_species(
    species: &Species,
    population: &[Genome],
    offspring_count: usize,
    innov: &mut InnovationTracker,
    cfg: &EvolutionConfig,
) -> Vec<Genome> {
    let mut rng = rand::rng();
    let mut offspring = Vec::new();

    if species.members.len() == 1 {
        let parent = &population[species.members[0]];
        for _ in 0..offspring_count {
            let mut child = Genome::new(
                parent.nodes.values().cloned().map(Some).collect(),
                parent.connections.clone(),
            );
            child.mutate(innov, cfg);
            offspring.push(child);
        }
    } else {
        for _ in 0..offspring_count {
            let parent_indices: Vec<&usize> = species.members.choose_multiple(&mut rng, 2).collect();
            let (mut parent1, mut parent2) = (&population[*parent_indices[0]], &population[*parent_indices[1]]);

            if parent1.fitness < parent2.fitness {
                std::mem::swap(&mut parent1, &mut parent2);
            }

            let mut child = crossover(parent1, parent2);
            child.mutate(innov, cfg);
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
    cfg: &EvolutionConfig,
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
            if s.stagnant_generations < cfg.stagnation_limit as u32 {
                surviving_species.push(s);
            }
        }
    }

    //println!("Species that survived: {}", surviving_species.len());

    let total_adjusted_fitness: f32 = surviving_species.iter().map(|s| s.adjusted_fitness).sum();
    //println!("Total adjusted fitness: {}", total_adjusted_fitness);

    for species in surviving_species.iter() {
        if !species.members.is_empty() {
            if let Some(&best_idx) = species
                .members
                .iter()
                .max_by(|&&a, &&b| population[a].fitness.total_cmp(&population[b].fitness))
            {
                new_population.push(population[best_idx].clone());
            }
        }
    }

    let remaining_offspring = population.len() - new_population.len();

    if remaining_offspring > 0 && !surviving_species.is_empty() {
        // Allocate offspring counts per species using floor + remainder to avoid overshoot
        let mut base_counts: Vec<usize> = Vec::with_capacity(surviving_species.len());
        let mut remainders: Vec<(usize, f32)> = Vec::with_capacity(surviving_species.len());

        if total_adjusted_fitness > 0.0 {
            for (i, s) in surviving_species.iter().enumerate() {
                let exact = (s.adjusted_fitness / total_adjusted_fitness) * remaining_offspring as f32;
                let base = exact.floor() as usize;
                base_counts.push(base);
                remainders.push((i, exact - base as f32));
            }
        } else {
            // Equal split when adjusted fitness is zero
            let equal = (remaining_offspring / surviving_species.len().max(1)) as usize;
            for i in 0..surviving_species.len() {
                base_counts.push(equal);
                remainders.push((i, 0.0));
            }
        }

        // Distribute leftover offspring according to largest remainders
        let used: usize = base_counts.iter().sum();
        let mut leftover = remaining_offspring.saturating_sub(used);
        remainders.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        for (i, _) in remainders.iter().copied() {
            if leftover == 0 { break; }
            base_counts[i] = base_counts[i].saturating_add(1);
            leftover -= 1;
        }

        // Reproduce according to final counts
        for (s, &count) in surviving_species.iter().zip(base_counts.iter()) {
            if count > 0 {
                let offspring = reproduce_species(s, &population, count, innov, cfg);
                new_population.extend(offspring);
            }
        }
    }

    // If we still need to top-up due to rounding edge cases, use the best species
    while new_population.len() < population.len() {
        if let Some(best_species) = surviving_species
            .iter()
            .max_by(|a, b| a.adjusted_fitness.total_cmp(&b.adjusted_fitness))
        {
            let offspring = reproduce_species(best_species, &population, 1, innov, cfg);
            new_population.extend(offspring);
        } else {
            break;
        }
    }

    // Enforce exact population size (truncate any accidental overshoot)
    if new_population.len() > population.len() {
        new_population.truncate(population.len());
    }

    new_population
}
