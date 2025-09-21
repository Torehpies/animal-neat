use std::f32;

use crate::neat::genome::Genome;

#[derive(Debug, Clone)]
pub struct Species {
    // Indices into the population vector
    pub representative: usize,
    pub members: Vec<usize>,
    pub adjusted_fitness: f32,
    pub best_fitness: f32,
    pub stagnant_generations: u32,
}

impl Species {
    pub fn new(representative_index: usize) -> Self {
        Self {
            representative: representative_index,
            members: vec![representative_index],
            adjusted_fitness: 0.0,
            best_fitness: f32::NEG_INFINITY,
            stagnant_generations: 0,
        }
    }

    pub fn add_member(&mut self, index: usize) {
        self.members.push(index);
    }

    pub fn clear_members(&mut self) {
        self.members.clear();
    }

    pub fn update_fitness_status(&mut self, population: &[Genome]) {
        if self.members.is_empty() {
            self.adjusted_fitness = 0.0;
            return;
        }

        let current_best_fitness = self
            .members
            .iter()
            .map(|&i| population[i].fitness)
            .fold(f32::NEG_INFINITY, f32::max);

        if current_best_fitness > self.best_fitness {
            self.best_fitness = current_best_fitness;
            self.stagnant_generations = 0;
        } else {
            self.stagnant_generations += 1;
        }

        let total_fitness: f32 = self.members.iter().map(|&i| population[i].fitness).sum();
        self.adjusted_fitness = total_fitness / self.members.len() as f32;
    }
}
