use crate::neat::connection_gene::ConnectionGene;
use crate::neat::genome::Genome;
use std::collections::{HashMap, HashSet};

pub fn distance(genome1: &Genome, genome2: &Genome, c1: f32, c2: f32, c3: f32) -> f32 {
    let genes1: HashMap<u32, &ConnectionGene> =
        genome1.connections.iter().map(|g| (g.innov, g)).collect();

    let genes2: HashMap<u32, &ConnectionGene> =
        genome2.connections.iter().map(|g| (g.innov, g)).collect();

    let innovations1: HashSet<u32> = genes1.keys().cloned().collect();
    let innovations2: HashSet<u32> = genes2.keys().cloned().collect();

    let matching: HashSet<u32> = innovations1.intersection(&innovations2).cloned().collect();
    let mut disjoint: HashSet<u32> = innovations1
        .symmetric_difference(&innovations2)
        .cloned()
        .collect();
    let mut excess: HashSet<u32> = HashSet::new();

    let max_innov1 = innovations1.iter().max().cloned().unwrap_or(0);
    let max_innov2 = innovations2.iter().max().cloned().unwrap_or(0);
    let max_innov = max_innov1.min(max_innov2);

    for innov in disjoint.clone() {
        if innov > max_innov {
            excess.insert(innov);
            disjoint.remove(&innov);
        }
    }

    let avg_weight_diff = if !matching.is_empty() {
        let weight_diff: f32 = matching
            .iter()
            .map(|i| (genes1[i].weight - genes2[i].weight).abs())
            .sum();
        weight_diff / matching.len() as f32
    } else {
        0.0
    };

    let mut n = genome1.connections.len().max(genome2.connections.len());
    if n < 20 {
        n = 1;
    }

    (c1 * excess.len() as f32) / n as f32
        + (c2 * disjoint.len() as f32) / n as f32
        + c3 * avg_weight_diff
}
