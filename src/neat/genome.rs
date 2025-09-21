use rand::seq::SliceRandom;
use rand::Rng;
use rand_distr;
use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use std::cell::RefCell;

use crate::neat::connection_gene::ConnectionGene;
use crate::neat::innovation_tracker::InnovationTracker;
use crate::neat::node_gene::{ActivationFunction, NodeGene, NodeType};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Genome {
    pub nodes: HashMap<u32, NodeGene>,
    pub connections: Vec<ConnectionGene>,
    pub fitness: f32,
    #[serde(skip)]
    cached: RefCell<Option<CachedGraph>>,
}

#[derive(Clone, Debug)]
struct CachedGraph {
    // Map node id <-> index
    id_to_index: HashMap<u32, usize>,
    index_to_id: Vec<u32>,
    // Topologically sorted node indices
    order: Vec<usize>,
    // For each node index, list of (incoming node index, connection index) for enabled edges
    incoming: Vec<Vec<(usize, usize)>>,
    // Stable, sorted input/output indices (by node id)
    input_indices: Vec<usize>,
    output_indices: Vec<usize>,
}

impl Genome {
    pub fn new(nodes: Vec<Option<NodeGene>>, connections: Vec<ConnectionGene>) -> Self {
        let mut node_map = HashMap::new();
        for node in nodes.into_iter().flatten() {
            node_map.insert(node.id, node);
        }

        Self {
            nodes: node_map,
            connections,
            fitness: 0.0,
            cached: RefCell::new(None),
        }
    }

    pub fn create_initial_genome(
        num_inputs: u32,
        num_outputs: u32,
        innov: &mut InnovationTracker,
    ) -> Self {
        let mut rng = rand::rng();

        let mut nodes: Vec<Option<NodeGene>> = Vec::new();
        let mut connections: Vec<ConnectionGene> = Vec::new();

        for i in 0..num_inputs {
            let node = NodeGene::new(i, NodeType::Input, ActivationFunction::Linear, 0.0);
            nodes.push(Some(node));
        }

        for i in 0..num_outputs {
            let node_id = num_inputs + i;
            let node = NodeGene::new(
                node_id,
                NodeType::Output,
                ActivationFunction::Sigmoid,
                rng.random_range(-1.0..1.0),
            );
            nodes.push(Some(node));
        }

        innov.node_id_counter = num_inputs + num_outputs;

        for i in 0..num_inputs {
            for j in 0..num_outputs {
                let in_node_id = i;
                let out_node_id = num_inputs + j;

                let innov_num = innov.get_connection_innovation(in_node_id, out_node_id);
                let weight = rng.random_range(-1.0..1.0);

                let conn = ConnectionGene {
                    in_node_id,
                    out_node_id,
                    weight,
                    enabled: true,
                    innov: innov_num,
                };

                connections.push(conn);
            }
        }
        Genome::new(nodes, connections)
    }

    pub fn create_initial_population(
        size: usize,
        num_inputs: u32,
        num_outputs: u32,
        innov: &mut InnovationTracker,
    ) -> Vec<Genome> {
        let mut population = Vec::with_capacity(size);
        for _ in 0..size {
            let genome = Genome::create_initial_genome(num_inputs, num_outputs, innov);
            population.push(genome);
        }
        population
    }

    pub fn evaluate(&self, input_values: Vec<f32>) -> Vec<f32> {
        self.evaluate_slice(&input_values)
    }

    pub fn evaluate_slice(&self, input_values: &[f32]) -> Vec<f32> {
        // Build caches if needed
        if self.cached.borrow().is_none() {
            let built = self.build_cache();
            *self.cached.borrow_mut() = Some(built);
        }
        let cache = self.cached.borrow();
        let cache = cache.as_ref().unwrap();

        if cache.input_indices.len() != input_values.len() {
            panic!(
                "Number of inputs doesn't match input nodes: got {}, expected {}",
                input_values.len(),
                cache.input_indices.len()
            );
        }

        let num_nodes = cache.index_to_id.len();
        let mut values = vec![0.0f32; num_nodes];

        // Set inputs in sorted order
        for (i, &idx) in cache.input_indices.iter().enumerate() {
            values[idx] = input_values[i];
        }

    for &idx in &cache.order {
            // Skip if already set (input nodes)
            if values[idx] != 0.0 {
                // It's possible a non-input computes to exactly 0.0; however inputs are assigned explicitly.
                // To robustly detect inputs, we could check node type; do that instead of value check.
            }
            let node_id = cache.index_to_id[idx];
            let node = &self.nodes[&node_id];
            if matches!(node.node_type, NodeType::Input) {
                continue;
            }

            let mut sum = 0.0f32;
            for &(in_idx, conn_idx) in &cache.incoming[idx] {
                let w = self.connections[conn_idx].weight;
                sum += values[in_idx] * w;
            }
            let value = node.activation.apply(sum + node.bias);
            values[idx] = value;
        }

        let mut outputs = Vec::with_capacity(cache.output_indices.len());
        for &idx in &cache.output_indices {
            outputs.push(values[idx]);
        }
        outputs
    }

    fn evaluate_uncached(&self, input_values: &[f32]) -> Vec<f32> {
        // Fallback: similar to cached but recompute structures quickly
        // Build id->index and list
        let mut ids: Vec<u32> = self.nodes.keys().copied().collect();
        ids.sort_unstable();
        let id_to_idx: HashMap<u32, usize> = ids
            .iter()
            .enumerate()
            .map(|(i, &id)| (id, i))
            .collect();

        let n = ids.len();
        let mut in_deg = vec![0usize; n];
        let mut edges: Vec<Vec<usize>> = vec![Vec::new(); n];
        let mut incoming: Vec<Vec<(usize, usize)>> = vec![Vec::new(); n];

        for (ci, c) in self.connections.iter().enumerate() {
            if !c.enabled {
                continue;
            }
            if let (Some(&u), Some(&v)) = (id_to_idx.get(&c.in_node_id), id_to_idx.get(&c.out_node_id)) {
                edges[u].push(v);
                in_deg[v] += 1;
                incoming[v].push((u, ci));
            }
        }

        let mut queue: std::collections::VecDeque<usize> = in_deg
            .iter()
            .enumerate()
            .filter_map(|(i, &d)| if d == 0 { Some(i) } else { None })
            .collect();
        let mut order = Vec::with_capacity(n);
        while let Some(u) = queue.pop_front() {
            order.push(u);
            for &v in &edges[u] {
                in_deg[v] -= 1;
                if in_deg[v] == 0 {
                    queue.push_back(v);
                }
            }
        }

        // Inputs/outputs sorted by id
        let mut input_indices: Vec<usize> = ids
            .iter()
            .enumerate()
            .filter_map(|(i, &id)| match self.nodes[&id].node_type {
                NodeType::Input => Some(i),
                _ => None,
            })
            .collect();
        input_indices.sort_unstable();
        let mut output_indices: Vec<usize> = ids
            .iter()
            .enumerate()
            .filter_map(|(i, &id)| match self.nodes[&id].node_type {
                NodeType::Output => Some(i),
                _ => None,
            })
            .collect();
        output_indices.sort_unstable();

        if input_indices.len() != input_values.len() {
            panic!(
                "Number of inputs doesn't match input nodes: got {}, expected {}",
                input_values.len(),
                input_indices.len()
            );
        }

        let mut values = vec![0.0f32; n];
        for (i, &idx) in input_indices.iter().enumerate() {
            values[idx] = input_values[i];
        }
        for &idx in &order {
            let node = &self.nodes[&ids[idx]];
            if matches!(node.node_type, NodeType::Input) {
                continue;
            }
            let mut sum = 0.0f32;
            for &(in_idx, ci) in &incoming[idx] {
                let w = self.connections[ci].weight;
                sum += values[in_idx] * w;
            }
            values[idx] = node.activation.apply(sum + node.bias);
        }
        output_indices.into_iter().map(|i| values[i]).collect()
    }

    fn build_cache(&self) -> CachedGraph {
        // Stable node id ordering
        let mut ids: Vec<u32> = self.nodes.keys().copied().collect();
        ids.sort_unstable();
        let id_to_index: HashMap<u32, usize> = ids
            .iter()
            .enumerate()
            .map(|(i, &id)| (id, i))
            .collect();

        let n = ids.len();
        let mut in_deg = vec![0usize; n];
        let mut edges: Vec<Vec<usize>> = vec![Vec::new(); n];
        let mut incoming: Vec<Vec<(usize, usize)>> = vec![Vec::new(); n];

        for (ci, c) in self.connections.iter().enumerate() {
            if !c.enabled {
                continue;
            }
            if let (Some(&u), Some(&v)) = (id_to_index.get(&c.in_node_id), id_to_index.get(&c.out_node_id)) {
                edges[u].push(v);
                in_deg[v] += 1;
                incoming[v].push((u, ci));
            }
        }

        let mut queue: std::collections::VecDeque<usize> = in_deg
            .iter()
            .enumerate()
            .filter_map(|(i, &d)| if d == 0 { Some(i) } else { None })
            .collect();
        let mut order = Vec::with_capacity(n);
        while let Some(u) = queue.pop_front() {
            order.push(u);
            for &v in &edges[u] {
                in_deg[v] -= 1;
                if in_deg[v] == 0 {
                    queue.push_back(v);
                }
            }
        }

        let mut input_indices: Vec<usize> = ids
            .iter()
            .enumerate()
            .filter_map(|(i, &id)| match self.nodes[&id].node_type {
                NodeType::Input => Some(i),
                _ => None,
            })
            .collect();
        input_indices.sort_unstable();
        let mut output_indices: Vec<usize> = ids
            .iter()
            .enumerate()
            .filter_map(|(i, &id)| match self.nodes[&id].node_type {
                NodeType::Output => Some(i),
                _ => None,
            })
            .collect();
        output_indices.sort_unstable();

        CachedGraph {
            id_to_index,
            index_to_id: ids,
            order,
            incoming,
            input_indices,
            output_indices,
        }
    }

    fn path_exists(
        &self,
        start_node_id: u32,
        end_node_id: u32,
        checked_nodes: &mut HashSet<u32>,
    ) -> bool {
        if start_node_id == end_node_id {
            return true;
        }

        checked_nodes.insert(start_node_id);

        for conn in &self.connections {
            if conn.enabled
                && conn.in_node_id == start_node_id
                && !checked_nodes.contains(&conn.out_node_id)
                && self.path_exists(conn.out_node_id, end_node_id, checked_nodes)
            {
                return true;
            }
        }
        false
    }

    pub fn get_node(&self, node_id: u32) -> Option<&NodeGene> {
        self.nodes.get(&node_id)
    }

    pub fn mutate_add_connection(&mut self, innov: &mut InnovationTracker) {
        let mut rng = rand::rng();
        let node_list: Vec<&NodeGene> = self.nodes.values().collect();

        let max_tries = 10;

        for _ in 0..max_tries {
            let (node1, node2) = {
                let mut sampled = node_list.clone();
                sampled.shuffle(&mut rng);
                let a = sampled[0];
                let b = sampled[1];

                if matches!(a.node_type, NodeType::Output)
                    || (matches!(a.node_type, NodeType::Hidden)
                        && matches!(b.node_type, NodeType::Input))
                {
                    (b, a)
                } else {
                    (a, b)
                }
            };

            if node1.id == node2.id {
                continue;
            }

            //            if node1.node_type == node2.node_type {
            //                continue;
            //            }

            if matches!(node1.node_type, NodeType::Output)
                || matches!(node2.node_type, NodeType::Input)
            {
                continue;
            }

            if self.connections.iter().any(|c| {
                (c.in_node_id == node1.id && c.out_node_id == node2.id)
                    || (c.in_node_id == node2.id && c.out_node_id == node1.id)
            }) {
                continue;
            }

            let mut checked = HashSet::new();

            if self.path_exists(node2.id, node1.id, &mut checked) {
                continue;
            }

            let innov_num = innov.get_connection_innovation(node1.id, node2.id);
            let weight = rng.random_range(-1.0..1.0);
            let new_conn = ConnectionGene::new(node1.id, node2.id, weight, true, innov_num);
            self.connections.push(new_conn);
            *self.cached.borrow_mut() = None; // topology changed
            return;
        }
    }

    pub fn mutate_add_node(&mut self, innov: &mut InnovationTracker) {
        let mut rng = rand::rng();

        let enabled_indices: Vec<usize> = self
            .connections
            .iter()
            .enumerate()
            .filter(|(_, c)| c.enabled)
            .map(|(i, _)| i)
            .collect();

        if enabled_indices.is_empty() {
            return;
        }

        let pick_index = rng.random_range(0..enabled_indices.len());
        let connection = &mut self.connections[enabled_indices[pick_index]];

        connection.enabled = false;

        let (node_id, conn1_innov, conn2_innov) = innov.get_node_innovation(connection.innov);

        let new_node = NodeGene::new(
            node_id,
            NodeType::Hidden,
            ActivationFunction::ReLU,
            rng.random_range(-1.0..1.0),
        );

        let conn1 = ConnectionGene::new(connection.in_node_id, node_id, 1.0, true, conn1_innov);
        let conn2 = ConnectionGene::new(
            node_id,
            connection.out_node_id,
            connection.weight,
            true,
            conn2_innov,
        );

        self.nodes.insert(node_id, new_node);
        self.connections.push(conn1);
        self.connections.push(conn2);
        *self.cached.borrow_mut() = None; // topology changed
    }

    pub fn mutate_weights(&mut self, rate: f32, power: f32, reinit_prob: f32) {
        let mut rng = rand::rng();
        for conn in &mut self.connections {
            if rng.random_bool(rate as f64) {
                if rng.random_bool(reinit_prob as f64) {
                    conn.weight = rng.random_range(-1.0..1.0);
                } else {
                    let pertrub: f32 = rng.sample(rand_distr::Normal::new(0.0, power).unwrap());
                    conn.weight += pertrub;
                    conn.weight = conn.weight.clamp(-5.0, 5.0);
                }
            }
        }
    }

    pub fn mutate_bias(&mut self, rate: f32, power: f32, reinit_prob: f32) {
        let mut rng = rand::rng();
        for node in self.nodes.values_mut() {
            if !matches!(node.node_type, NodeType::Input) && rng.random_bool(rate as f64) {
                if rng.random_bool(reinit_prob as f64) {
                    node.bias = rng.random_range(-1.0..1.0);
                } else {
                    let pertrub: f32 = rng.sample(rand_distr::Normal::new(0.0, power).unwrap());
                    node.bias += pertrub;
                    node.bias = node.bias.clamp(-5.0, 5.0);
                }
            }
        }
    }

    pub fn mutate(&mut self, innov: &mut InnovationTracker, cfg: &crate::neat::config::EvolutionConfig) {
        self.mutate_weights(cfg.weight_mutation_rate, cfg.weight_perturb_power, cfg.reinit_weight_prob);
        self.mutate_bias(cfg.bias_mutation_rate, cfg.bias_perturb_power, cfg.reinit_bias_prob);

        let mut rng = rand::rng();
        if rng.random_bool(cfg.conn_mutation_rate as f64) {
            //println!("Mutation: add connection");
            self.mutate_add_connection(innov);
        }
        if rng.random_bool(cfg.node_mutation_rate as f64) {
            //println!("Mutation: add node");
            self.mutate_add_node(innov);
        }
    }
}

impl PartialEq for Genome {
    fn eq(&self, other: &Self) -> bool {
        self.nodes == other.nodes && self.connections == other.connections && self.fitness == other.fitness
    }
}

// Note: The cached evaluation path avoids this generic topological_sort; retained only for reference
