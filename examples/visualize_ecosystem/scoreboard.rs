use crate::sim;
use crate::sim::Agent;
use crate::params::*;

#[derive(Clone, Debug)]
pub struct ScoreEntry {
    pub idx: usize,
    pub species: usize,
    pub score: f32,
    pub eaten_plants: i32,
    pub eaten_meat: i32,
    pub offspring: usize,
    pub alive_steps: u32,
    pub idle_penalty_value: f32,
    pub herd_value: f32,
    pub approach_value: f32,
    pub chase_value: f32,
    pub chase_same_value: f32,
    pub flee_value: f32,
    pub rest_content_value: f32,
    pub eat_early_value: f32,
    pub attack_hits: usize,
    pub kills_caused: usize,
    pub avg_energy_norm: f32,
}

// Compute episode scores similar to eco_cull, without side effects
pub fn compute_episode_score(a: &Agent, comm_fit: f32) -> (f32, i32, i32, f32, f32, f32, f32, f32, f32, f32, f32, f32, usize, usize, f32) {
    use crate::params::*;
    // Scoreboard score aligns with eval.rs weighted formula
    let kind = match a.kind { sim::AgentKind::Herbivore => Kind::Herb, sim::AgentKind::Carnivore => Kind::Carn };
    let avg_energy_norm = if a.alive_steps > 0 { (a.energy_accum / a.alive_steps as f32) / crate::params::get_max_energy_for(kind) } else { 0.0 };
    let lifetime_score = (a.alive_steps as f32) / (MAX_STEPS as f32);
    let offspring_score = a.offspring_count as f32;
    let plants = a.eaten.saturating_sub(a.kills) as f32;
    let meat = a.kills as f32;
    // Normalize idle penalty so w_idle meaning is comparable to other [0,1] terms
    let max_idle_steps = (MAX_STEPS.saturating_sub(IDLENESS_THRESHOLD_STEPS)) as f32;
    let max_idle_penalty = max_idle_steps * IDLENESS_PENALTY_PER_STEP;
    let idle_penalty = if max_idle_penalty > 0.0 { (a.total_idle_penalty / max_idle_penalty).clamp(0.0, 1.0) } else { 0.0 };
    let w_life = crate::params::get_fit_lifetime_weight(kind);
    let w_energy = crate::params::get_fit_energy_weight(kind);
    let w_off = crate::params::get_fit_offspring_weight(kind);
    let w_comm = crate::params::get_fit_comm_weight(kind);
    let w_idle = crate::params::get_fit_idle_penalty_weight(kind);
    let w_plant = crate::params::get_fit_plant_weight(kind);
    let w_meat = crate::params::get_fit_meat_weight(kind);
    let w_att = crate::params::get_fit_attacks_weight(kind);
    let w_kill = crate::params::get_fit_kills_weight(kind);
    let w_herd = crate::params::get_fit_herding_weight(kind);
    let w_approach = crate::params::get_fit_approach_food_weight(kind);
    let w_chase = crate::params::get_fit_chase_other_weight(kind);
    let w_chase_same = crate::params::get_fit_chase_same_weight(kind);
    let w_restc = crate::params::get_fit_rest_content_weight(kind);
    let w_eearly = crate::params::get_fit_eat_early_weight(kind);
    let w_flee = crate::params::get_fit_flee_other_weight(kind);
    let score = w_life * lifetime_score
        + w_energy * avg_energy_norm
        + w_off * offspring_score
        + w_comm * comm_fit
        - if IDLENESS_PENALTY_ENABLED { w_idle * idle_penalty } else { 0.0 }
        + w_plant * plants
        + w_meat * meat
        + w_att * (a.attack_hits as f32)
        + w_kill * (a.kills_caused as f32)
        + w_herd * a.herding_units
        + w_approach * a.approach_food_units
        + w_chase * a.chase_other_units
        + w_chase_same * a.chase_same_units
    + w_restc * a.rest_content_units
    + w_flee * a.flee_other_units
        + w_eearly * a.eat_early_units;
    // Keep plant/meat counts for display only
    let eaten_plants = a.eaten.saturating_sub(a.kills) as i32;
    let eaten_meat = a.kills as i32;
    let idle_penalty_value = if IDLENESS_PENALTY_ENABLED { w_idle * idle_penalty } else { 0.0 };
    let herd_value = w_herd * a.herding_units;
    let approach_value = w_approach * a.approach_food_units;
    let chase_value = w_chase * a.chase_other_units;
    let chase_same_value = w_chase_same * a.chase_same_units;
    let flee_value = w_flee * a.flee_other_units;
    let rest_content_value = w_restc * a.rest_content_units;
    let eat_early_value = w_eearly * a.eat_early_units;
    (score, eaten_plants, eaten_meat, avg_energy_norm, idle_penalty_value, herd_value, approach_value, chase_value, chase_same_value, flee_value, rest_content_value, eat_early_value, a.attack_hits, a.kills_caused, avg_energy_norm)
}

pub fn prepare_scoreboard(state: &mut crate::AppState) {
    // Build rows and sort by score desc
    let mut rows: Vec<ScoreEntry> = Vec::with_capacity(state.episode.agents.len());
    for (i, a) in state.episode.agents.iter().enumerate() {
        let comm_fit = state.episode.comm_fitness_accum.get(i).copied().unwrap_or(0.0);
    let (score, plants, meat, avg_energy_norm, idle_penalty_value, herd_value, approach_value, chase_value, chase_same_value, flee_value, rest_content_value, eat_early_value, _attack_hits, _kills_caused, _avg_energy_repeat) = compute_episode_score(a, comm_fit);
        rows.push(ScoreEntry {
            idx: i,
            species: a.species_id,
            score,
            eaten_plants: plants,
            eaten_meat: meat,
            offspring: a.offspring_count,
            alive_steps: a.alive_steps,
            idle_penalty_value,
            herd_value,
            approach_value,
            chase_value,
            chase_same_value,
            flee_value,
            rest_content_value,
            eat_early_value,
            attack_hits: a.attack_hits,
            kills_caused: a.kills_caused,
            avg_energy_norm,
        });
    }
    rows.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    state.scoreboard_rows = rows;
    state.scoreboard_pending = true; // scoreboard is open now; rendering is gated on show_scoreboard_panel
}
