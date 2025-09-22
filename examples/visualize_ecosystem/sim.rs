use super::params::*;
use super::{Agent, DigestEvent, Vec2};

// Apply digestion for one agent, returning energy gained this tick
pub fn apply_digestion(agent: &mut Agent) {
    if agent.digest.is_empty() { return; }
    let mut gained = 0.0f32;
    for ev in agent.digest.iter_mut() {
        if ev.remaining > 0 { gained += ev.per_step; ev.remaining -= 1; }
    }
    agent.digest.retain(|ev| ev.remaining > 0);
    agent.energy = (agent.energy + gained).min(INITIAL_ENERGY);
}

// Resolve predation/scavenging interactions using a snapshot of positions and life states
pub fn resolve_predation(
    agents: &mut [Agent],
    prey_targets: &[Option<usize>],
    step_idx: usize,
) {
    let mut claimed = vec![false; agents.len()];
    for i in 0..agents.len() {
        if let Some(j) = prey_targets[i] {
            if claimed[j] || agents[j].consumed { continue; }
            let alive_j = agents[j].energy > 0.0;
            if (alive_j && !PREDATION_ENABLED) || (!alive_j && !SCAVENGE_ENABLED) { continue; }
            let gain = if alive_j { CORPSE_INITIAL_ENERGY } else { agents[j].corpse_energy.max(0.0) };
            agents[j].energy = 0.0;
            agents[j].dead_since.get_or_insert(step_idx);
            agents[j].corpse_energy = 0.0;
            agents[j].consumed = true;
            if gain > 0.0 {
                if DIGEST_STEPS_MEAT > 0 { agents[i].digest.push_back(DigestEvent { remaining: DIGEST_STEPS_MEAT, per_step: gain / (DIGEST_STEPS_MEAT as f32) }); }
                else { agents[i].energy = (agents[i].energy + gain).min(INITIAL_ENERGY); }
            }
            agents[i].eaten += 1;
            agents[i].kills += 1;
            agents[i].predation_flash_steps = agents[i].predation_flash_steps.saturating_add(10);
            claimed[j] = true;
        }
    }
}

pub fn decay_corpses_and_flashes(agents: &mut [Agent]) {
    for a in agents.iter_mut() {
        if a.energy <= 0.0 && !a.consumed && a.corpse_energy > 0.0 {
            a.corpse_energy *= (1.0 - CORPSE_DECAY_RATE).max(0.0);
            if a.corpse_energy < 0.1 { a.corpse_energy = 0.0; a.consumed = true; }
        }
        if a.predation_flash_steps > 0 { a.predation_flash_steps -= 1; }
    }
}
