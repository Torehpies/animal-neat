use crate::neat::genome::Genome;
use std::io;

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
