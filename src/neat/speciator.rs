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

        for genome in population {
            let mut found_species = false;

            for species in &mut self.species {
                if distance(genome, &species.representative, 1.0, 1.0, 0.4)
                    < self.compatibility_threshold
                {
                    species.add_member(genome.clone());
                    found_species = true;
                    break;
                }
            }

            if !found_species {
                let new_species = Species::new(genome.clone());
                self.species.push(new_species);
            }
        }

        self.species.retain(|s| !s.members.is_empty());

        for species in &mut self.species {
            species.update_fitness_status();

            if let Some(best) = species
                .members
                .iter()
                .cloned()
                .max_by(|a, b| a.fitness.total_cmp(&b.fitness))
            {
                species.representative = best;
            }
        }
    }

    pub fn get_species(&self) -> &Vec<Species> {
        &self.species
    }

    pub fn get_species_mut(&mut self) -> &mut Vec<Species> {
        &mut self.species
    }
}
