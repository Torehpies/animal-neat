use crate::neat::{genome::Genome, innovation_tracker::InnovationTracker};

#[derive(Debug)]
pub struct Population {
    pub genomes: Vec<Genome>,
}

impl Population {
    pub fn new(
        size: usize,
        num_inputs: u32,
        num_outputs: u32,
        innov: &mut InnovationTracker,
    ) -> Self {
        let mut genomes = Vec::new();
        for _ in 0..size {
            let genome = Genome::create_initial_genome(num_inputs, num_outputs, innov);
            genomes.push(genome);
        }
        Self { genomes }
    }
}
