use std::path::PathBuf;

use neat::neat::{genome::Genome, innovation_tracker::InnovationTracker, io};

#[test]
fn genome_json_round_trip_and_eval_shape() {
    let mut innov = InnovationTracker::new();
    let num_inputs = 2;
    let num_outputs = 1;

    let genome = Genome::create_initial_genome(num_inputs, num_outputs, &mut innov);

    // Round-trip JSON
    let mut path = PathBuf::from(std::env::temp_dir());
    path.push(format!("neat_test_{}.json", std::process::id()));

    io::save_genome_json(path.to_str().unwrap(), &genome).expect("save genome json");
    let loaded = io::load_genome_json(path.to_str().unwrap()).expect("load genome json");

    assert_eq!(genome, loaded);

    // Evaluate shape
    let out = loaded.evaluate(vec![0.0, 1.0]);
    assert_eq!(out.len(), num_outputs as usize);

    // Cleanup best-effort
    let _ = std::fs::remove_file(path);
}
