use std::collections::{HashMap, HashSet};

use rand::Rng;

use crate::neat::{
    connection_gene::ConnectionGene,
    genome::Genome,
    node_gene::{NodeGene, NodeType},
};

pub fn crossover(parent1: &Genome, parent2: &Genome) -> Genome {
    let mut rng = rand::rng();

    let mut offspring_connections: Vec<ConnectionGene> = Vec::new();
    let mut offspring_node_ids: HashSet<u32> = HashSet::new();
    let mut all_nodes: HashMap<u32, NodeGene> = HashMap::new();

    for node in parent1.nodes.values() {
        all_nodes.insert(node.id, node.clone());

        if matches!(node.node_type, NodeType::Input | NodeType::Output) {
            offspring_node_ids.insert(node.id);
        }
    }

    for node in parent2.nodes.values() {
        all_nodes.entry(node.id).or_insert_with(|| node.clone());
    }

    let genes1: HashMap<u32, ConnectionGene> = parent1
        .connections
        .iter()
        .map(|g| (g.innov, g.clone()))
        .collect();
    let genes2: HashMap<u32, ConnectionGene> = parent2
        .connections
        .iter()
        .map(|g| (g.innov, g.clone()))
        .collect();

    let mut all_innovs: Vec<u32> = genes1.keys().chain(genes2.keys()).cloned().collect();
    all_innovs.sort_unstable();
    all_innovs.dedup();

    for innov in all_innovs {
        let gene1 = genes1.get(&innov);
        let gene2 = genes2.get(&innov);

        let gene_copy: Option<ConnectionGene>;

        if let (Some(g1), Some(g2)) = (gene1, gene2) {
            let selected = if rng.random_bool(0.5) { g1 } else { g2 };
            let mut new_gene = selected.clone();

            if (!g1.enabled || !g2.enabled) && rng.random_bool(0.75) {
                new_gene.enabled = false;
            }
            gene_copy = Some(new_gene);
        } else if let Some(g1) = gene1 {
            gene_copy = Some(g1.clone());
        } else {
            continue;
        }
        if let Some(g) = gene_copy {
            if all_nodes.contains_key(&g.in_node_id) && all_nodes.contains_key(&g.out_node_id) {
                offspring_connections.push(g.clone());
                offspring_node_ids.insert(g.in_node_id);
                offspring_node_ids.insert(g.out_node_id);
            }
        }
    }

    let nodes_vec: Vec<Option<NodeGene>> = offspring_node_ids
        .into_iter()
        .filter_map(|id| all_nodes.get(&id).cloned())
        .map(Some)
        .collect();

    Genome::new(nodes_vec, offspring_connections)
}
