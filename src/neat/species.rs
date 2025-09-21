use std::f32;

use crate::neat::genome::Genome;

#[derive(Debug, Clone)]
pub struct Species {
    pub representative: Genome,
    pub members: Vec<Genome>,
    pub adjusted_fitness: f32,
    pub best_fitness: f32,
    pub stagnant_generations: u32,
}

impl Species {
    pub fn new(representative: Genome) -> Self {
        Self {
            representative: representative.clone(),
            members: vec![representative],
            adjusted_fitness: 0.0,
            best_fitness: f32::NEG_INFINITY,
            stagnant_generations: 0,
        }
    }

    pub fn add_member(&mut self, genome: Genome) {
        self.members.push(genome);
    }

    pub fn clear_members(&mut self) {
        self.members.clear();
    }

    pub fn update_fitness_status(&mut self) {
        if self.members.is_empty() {
            self.adjusted_fitness = 0.0;
            return;
        }

        let current_best_fitness = self
            .members
            .iter()
            .map(|m| m.fitness)
            .fold(f32::NEG_INFINITY, f32::max);

        if current_best_fitness > self.best_fitness {
            self.best_fitness = current_best_fitness;
            self.stagnant_generations = 0;
        } else {
            self.stagnant_generations += 1;
        }

        let total_fitness: f32 = self.members.iter().map(|m| m.fitness).sum();
        self.adjusted_fitness = total_fitness / self.members.len() as f32;
    }
}
