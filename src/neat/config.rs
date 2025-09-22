#[derive(Debug, Clone)]
pub struct EvolutionConfig {
    pub conn_mutation_rate: f32,
    pub node_mutation_rate: f32,
    pub weight_mutation_rate: f32,
    pub bias_mutation_rate: f32,
    pub weight_perturb_power: f32,
    pub bias_perturb_power: f32,
    pub reinit_weight_prob: f32,
    pub reinit_bias_prob: f32,
    pub stagnation_limit: usize,
    pub compatibility_threshold: f32,
}

impl Default for EvolutionConfig {
    fn default() -> Self {
        Self {
            conn_mutation_rate: 0.12,
            node_mutation_rate: 0.05,
            weight_mutation_rate: 0.9,
            bias_mutation_rate: 0.7,
            weight_perturb_power: 0.5,
            bias_perturb_power: 0.5,
            reinit_weight_prob: 0.1,
            reinit_bias_prob: 0.1,
            stagnation_limit: 15,
            compatibility_threshold: 2.0,
        }
    }
}
