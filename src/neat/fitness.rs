use crate::neat::genome::Genome;

pub fn fitness_xor(genome: &Genome) -> f32 {
    let x_data = vec![
        vec![0.0, 0.0],
        vec![0.0, 1.0],
        vec![1.0, 0.0],
        vec![1.0, 1.0],
    ];

    let y_data = vec![0.0, 1.0, 1.0, 0.0];

    let mut total_error = 0.0;

    for (inputs, target) in x_data.into_iter().zip(y_data.into_iter()) {
        let output = genome.evaluate(inputs);

        if let Some(&val) = output.first() {
            let error = (val - target).abs();
            total_error += error;
        } else {
            total_error += target;
        }
    }

    let fitness = 4.0 - total_error;
    fitness.max(0.0)
}
