use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;
use serde::de::Error as DeError;

// Helper to serialize/deserialize HashMap<(u32,u32), u32> as a Vec of ((u32,u32), u32)
mod conn_map_serde {
    use super::*;

    pub fn serialize<S>(map: &HashMap<(u32, u32), u32>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let vec: Vec<((u32, u32), u32)> = map.iter().map(|(k, v)| (*k, *v)).collect();
        vec.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<HashMap<(u32, u32), u32>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let vec: Vec<((u32, u32), u32)> = Vec::deserialize(deserializer)?;
        let mut map = HashMap::new();
        for (k, v) in vec {
            map.insert(k, v);
        }
        Ok(map)
    }
}

// Helper to serialize/deserialize HashMap<u32, (u32,u32,u32)> as Vec of (u32, (u32,u32,u32))
mod node_map_serde {
    use super::*;

    pub fn serialize<S>(map: &HashMap<u32, (u32, u32, u32)>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let vec: Vec<(u32, (u32, u32, u32))> = map.iter().map(|(k, v)| (*k, *v)).collect();
        vec.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<HashMap<u32, (u32, u32, u32)>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let vec: Vec<(u32, (u32, u32, u32))> = Vec::deserialize(deserializer)?;
        let mut map = HashMap::new();
        for (k, v) in vec {
            map.insert(k, v);
        }
        Ok(map)
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct InnovationTracker {
    pub current_innovation: u32,
    #[serde(with = "conn_map_serde")]
    pub connection_innovations: HashMap<(u32, u32), u32>,
    #[serde(with = "node_map_serde")]
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
