use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct InnovationTracker {
    pub current_innovation: u32,
    pub connection_innovations: HashMap<(u32, u32), u32>,
    pub node_innovations: HashMap<u32, (u32, u32, u32)>,
    pub node_id_counter: u32,
}

impl InnovationTracker {
    pub fn new() -> Self {
        Self {
            current_innovation: 0,
            connection_innovations: HashMap::new(),
            node_innovations: HashMap::new(),
            node_id_counter: 0,
        }
    }

    pub fn get_connection_innovation(&mut self, in_node_id: u32, out_node_id: u32) -> u32 {
        let key = (in_node_id, out_node_id);

        let entry = self.connection_innovations.entry(key).or_insert_with(|| {
            let innov = self.current_innovation;
            self.current_innovation += 1;
            innov
        });

        *entry
    }

    pub fn get_node_innovation(&mut self, connection_innovation: u32) -> (u32, u32, u32) {
        let entry = self
            .node_innovations
            .entry(connection_innovation)
            .or_insert_with(|| {
                let node_id: u32 = self.node_id_counter;
                self.node_id_counter += 1;

                let conn1_innovation = self.current_innovation;
                self.current_innovation += 1;
                let conn2_innovation = self.current_innovation;
                self.current_innovation += 1;

                (node_id, conn1_innovation, conn2_innovation)
            });

        *entry
    }
}
