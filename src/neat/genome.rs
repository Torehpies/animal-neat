use rand::Rng;
use rand::seq::SliceRandom;
use rand_distr;
use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::neat::connection_gene::ConnectionGene;
use crate::neat::innovation_tracker::InnovationTracker;
use crate::neat::node_gene::{ActivationFunction, NodeGene, NodeType};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Genome {
    pub nodes: HashMap<u32, NodeGene>,
    pub connections: Vec<ConnectionGene>,
    pub fitness: f32,
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
        let mut node_values: HashMap<u32, f32> = HashMap::new();
        let mut node_inputs: HashMap<u32, Vec<&ConnectionGene>> =
            self.nodes.keys().map(|&n| (n, Vec::new())).collect();

        let input_nodes: Vec<&NodeGene> = self
            .nodes
            .values()
            .filter(|n| n.node_type == NodeType::Input)
            .collect();
        let output_nodes: Vec<&NodeGene> = self
            .nodes
            .values()
            .filter(|n| n.node_type == NodeType::Output)
            .collect();

        if input_nodes.len() != input_values.len() {
            panic!(
                "Number of inputs doesn't match input nodes: got {}, expected {}",
                input_values.len(),
                input_nodes.len()
            );
        }

        for (node, val) in input_nodes.iter().zip(input_values.into_iter()) {
            node_values.insert(node.id, val);
        }

        let mut edges: HashMap<u32, Vec<u32>> =
            self.nodes.keys().map(|&n| (n, Vec::new())).collect();

        for conn in &self.connections {
            if conn.enabled {
                if let Some(v) = edges.get_mut(&conn.in_node_id) {
                    v.push(conn.out_node_id);
                }
                if let Some(inputs) = node_inputs.get_mut(&conn.out_node_id) {
                    inputs.push(conn);
                }
            }
        }

        let sorted_nodes = topological_sort(&edges);

        for node_id in sorted_nodes {
            if node_values.contains_key(&node_id) {
                continue;
            }

            let incoming = &node_inputs[&node_id];
            let total_input: f32 = incoming
                .iter()
                .map(|c| node_values.get(&c.in_node_id).copied().unwrap_or(0.0) * c.weight)
                .sum();

            let node = &self.nodes[&node_id];
            let value = node.activation.apply(total_input + node.bias);

            node_values.insert(node_id, value);
        }

        output_nodes
            .iter()
            .map(|n| *node_values.get(&n.id).unwrap_or(&0.0))
            .collect()
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
    }

    pub fn mutate_weights(&mut self, rate: f32, power: f32) {
        let mut rng = rand::rng();
        for conn in &mut self.connections {
            if rng.random_bool(rate as f64) {
                if rng.random_bool(0.1) {
                    conn.weight = rng.random_range(-1.0..1.0);
                } else {
                    let pertrub: f32 = rng.sample(rand_distr::Normal::new(0.0, power).unwrap());
                    conn.weight += pertrub;
                    conn.weight = conn.weight.clamp(-5.0, 5.0);
                }
            }
        }
    }

    pub fn mutate_bias(&mut self, rate: f32, power: f32) {
        let mut rng = rand::rng();
        for node in self.nodes.values_mut() {
            if !matches!(node.node_type, NodeType::Input) && rng.random_bool(rate as f64) {
                if rng.random_bool(0.1) {
                    node.bias = rng.random_range(-1.0..1.0);
                } else {
                    let pertrub: f32 = rng.sample(rand_distr::Normal::new(0.0, power).unwrap());
                    node.bias += pertrub;
                    node.bias = node.bias.clamp(-5.0, 5.0);
                }
            }
        }
    }

    pub fn mutate(
        &mut self,
        innov: &mut InnovationTracker,
        conn_mutation_rate: f32,
        node_mutation_rate: f32,
        weight_mutation_rate: f32,
        bias_mutation_rate: f32,
    ) {
        self.mutate_weights(weight_mutation_rate, 0.5);
        self.mutate_bias(bias_mutation_rate, 0.5);

        let mut rng = rand::rng();
        if rng.random_bool(conn_mutation_rate as f64) {
            //println!("Mutation: add connection");
            self.mutate_add_connection(innov);
        }
        if rng.random_bool(node_mutation_rate as f64) {
            //println!("Mutation: add node");
            self.mutate_add_node(innov);
        }
    }
}

fn topological_sort(edges: &HashMap<u32, Vec<u32>>) -> Vec<u32> {
    fn visit(
        n: u32,
        visited: &mut HashSet<u32>,
        order: &mut Vec<u32>,
        edges: &HashMap<u32, Vec<u32>>,
    ) {
        if visited.contains(&n) {
            return;
        }
        visited.insert(n);

        if let Some(children) = edges.get(&n) {
            for &m in children {
                visit(m, visited, order, edges);
            }
        }

        order.push(n);
    }

    let mut visited = HashSet::new();
    let mut order = Vec::new();

    for &node in edges.keys() {
        visit(node, &mut visited, &mut order, edges);
    }

    order.reverse();
    order
}
