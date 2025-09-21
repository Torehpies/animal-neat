use neat::neat::{
    config::EvolutionConfig,
    evolution,
    genome::Genome,
    innovation_tracker::InnovationTracker,
    speciator::Speciator,
};

fn fitness_example(g: &Genome) -> f32 {
    // XOR-like example fitness (replace with your own problem!)
    const X: [[f32; 2]; 4] = [[0.0, 0.0], [0.0, 1.0], [1.0, 0.0], [1.0, 1.0]];
    const Y: [f32; 4] = [0.0, 1.0, 1.0, 0.0];

    let mut total_err = 0.0;
    for i in 0..4 {
        let out = g.evaluate_slice(&X[i]);
        total_err += (out.get(0).copied().unwrap_or(0.0) - Y[i]).abs();
    }
    (4.0 - total_err).max(0.0)
}

fn main() {
    let num_inputs = 2;
    let num_outputs = 1;
    let pop_size = 60;
    let generations = 100;
    let target = 3.9;

    let mut innov = InnovationTracker::new();
    let mut speciator = Speciator::new(2.0);

    let mut population = Genome::create_initial_population(pop_size, num_inputs, num_outputs, &mut innov);
    let cfg = EvolutionConfig { compatibility_threshold: 2.0, ..Default::default() };

    for step in 0..generations {
        let fitness_scores: Vec<f32> = population.iter().map(fitness_example).collect();

        let best = fitness_scores.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        let avg = fitness_scores.iter().sum::<f32>() / fitness_scores.len() as f32;
        println!("Gen {}: best={:.3}, avg={:.3}", step, best, avg);

        if best >= target {
            println!("Solved at generation {}", step);
            break;
        }

        population = evolution::evolution(population, fitness_scores, &mut speciator, &mut innov, &cfg);
    }
}
