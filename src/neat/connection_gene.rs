use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ConnectionGene {
    pub in_node_id: u32,
    pub out_node_id: u32,
    pub weight: f32,
    pub enabled: bool,
    pub innov: u32,
}

impl ConnectionGene {
    pub fn new(in_node_id: u32, out_node_id: u32, weight: f32, enabled: bool, innov: u32) -> Self {
        Self {
            in_node_id,
            out_node_id,
            weight,
            enabled,
            innov,
        }
    }
}
