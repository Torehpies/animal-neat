use crate::neat::genome::Genome;

const XOR_INPUTS: [[f32; 2]; 4] = [[0.0, 0.0], [0.0, 1.0], [1.0, 0.0], [1.0, 1.0]];
const XOR_TARGETS: [f32; 4] = [0.0, 1.0, 1.0, 0.0];

pub fn fitness_xor(genome: &Genome) -> f32 {
    let mut total_error = 0.0;
    for i in 0..4 {
        let output = genome.evaluate_slice(&XOR_INPUTS[i]);
        let val = output.first().copied().unwrap_or(0.0);
        let error = (val - XOR_TARGETS[i]).abs();
        total_error += error;
    }
    (4.0 - total_error).max(0.0)
}
