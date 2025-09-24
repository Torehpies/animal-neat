use crate::neat::{genome::Genome, innovation_tracker::InnovationTracker};
use std::io;

use serde::{Deserialize, Serialize};

/// Save a `Genome` to a JSON file at the given path.
///
/// Returns `io::Result<()>` with any filesystem or serialization errors propagated.
pub fn save_genome_json(path: &str, genome: &Genome) -> io::Result<()> {
    let s = serde_json::to_string_pretty(&genome).map_err(io::Error::other)?;
    std::fs::write(path, s)
}

/// Load a `Genome` from a JSON file at the given path.
///
/// Returns `io::Result<Genome>` with any filesystem or deserialization errors propagated.
pub fn load_genome_json(path: &str) -> io::Result<Genome> {
    let s = std::fs::read_to_string(path)?;
    let g: Genome = serde_json::from_str(&s).map_err(io::Error::other)?;
    Ok(g)
}

/// A full population snapshot including generation counter and innovation tracker state.
/// This enables resuming evolution or performing offline analysis of diversity/topologies.
#[derive(Clone, Serialize, Deserialize)]
pub struct PopulationSnapshot {
    pub generation: usize,
    pub population: Vec<Genome>,
    pub innovation: InnovationTracker,
}

/// Save the entire current population plus innovation tracker and generation to a JSON file.
pub fn save_population_snapshot(
    path: &str,
    generation: usize,
    population: &[Genome],
    innov: &InnovationTracker,
) -> io::Result<()> {
    let snap = PopulationSnapshot {
        generation,
        population: population.to_vec(),
        innovation: innov.clone(),
    };
    let s = serde_json::to_string_pretty(&snap).map_err(io::Error::other)?;
    if let Some(parent) = std::path::Path::new(path).parent() {
        if !parent.as_os_str().is_empty() { let _ = std::fs::create_dir_all(parent); }
    }
    std::fs::write(path, s)
}

/// Load a previously saved population snapshot.
pub fn load_population_snapshot(path: &str) -> io::Result<PopulationSnapshot> {
    let s = std::fs::read_to_string(path)?;
    let snap: PopulationSnapshot = serde_json::from_str(&s).map_err(io::Error::other)?;
    Ok(snap)
}
