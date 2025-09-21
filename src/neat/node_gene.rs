use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ActivationFunction {
    Sigmoid,
    ReLU,
    Tanh,
    Softmax,
    Linear,
}

impl ActivationFunction {
    pub fn apply(&self, x: f32) -> f32 {
        match self {
            ActivationFunction::Sigmoid => 1.0 / (1.0 + (-x).exp()),
            ActivationFunction::ReLU => {
                if x > 0.0 {
                    x
                } else {
                    0.0
                }
            }
            ActivationFunction::Tanh => x.tanh(),
            ActivationFunction::Softmax => x,
            ActivationFunction::Linear => x,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NodeType {
    Input,
    Hidden,
    Output,
    Bias,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NodeGene {
    pub id: u32,
    pub node_type: NodeType,
    pub activation: ActivationFunction,
    pub bias: f32,
}

impl NodeGene {
    pub fn new(id: u32, node_type: NodeType, activation: ActivationFunction, bias: f32) -> Self {
        Self {
            id,
            node_type,
            activation,
            bias,
        }
    }
}
