use crate::neat::{compatibility::distance, genome::Genome, species::Species};

#[derive(Debug)]
pub struct Speciator {
    pub species: Vec<Species>,
    pub compatibility_threshold: f32,
    pub target_species_count: usize,
    pub adjust_step: f32,
}

impl Speciator {
    pub fn new(compatibility_threshold: f32) -> Self {
        Self {
            species: Vec::new(),
            compatibility_threshold,
            target_species_count: 5,
            adjust_step: 0.05,
        }
    }

    pub fn with_target(mut self, target: usize, step: f32) -> Self {
        self.target_species_count = target;
        self.adjust_step = step;
        self
    }

    pub fn speciate(&mut self, population: &[Genome]) {
        for s in &mut self.species {
            s.clear_members();
        }

        for (i, genome) in population.iter().enumerate() {
            let mut found_species = false;

            for species in &mut self.species {
                let rep = &population[species.representative];
                if distance(genome, rep, 1.0, 1.0, 0.4) < self.compatibility_threshold {
                    species.add_member(i);
                    found_species = true;
                    break;
                }
            }

            if !found_species {
                let new_species = Species::new(i);
                self.species.push(new_species);
            }
        }

        self.species.retain(|s| !s.members.is_empty());

        for species in &mut self.species {
            species.update_fitness_status(population);

            if let Some(&best_idx) = species
                .members
                .iter()
                .max_by(|&&a, &&b| population[a].fitness.total_cmp(&population[b].fitness))
            {
                species.representative = best_idx;
            }
        }

        // Nudge the compatibility threshold toward the target species count
        let count = self.species.len();
        if count < self.target_species_count {
            // Too few species -> decrease threshold to split more easily
            self.compatibility_threshold = (self.compatibility_threshold - self.adjust_step).max(0.05);
        } else if count > self.target_species_count {
            // Too many species -> increase threshold to merge more easily
            self.compatibility_threshold = (self.compatibility_threshold + self.adjust_step).min(10.0);
        }
    }

    pub fn get_species(&self) -> &Vec<Species> {
        &self.species
    }

    pub fn get_species_mut(&mut self) -> &mut Vec<Species> {
        &mut self.species
    }
}
