use macroquad::prelude::*;

use crate::sim::{Agent, DigestEvent};
use crate::params::*;



// Apply digestion for one agent, returning energy gained this tick
pub fn apply_digestion(agent: &mut Agent) {
    if agent.digest.is_empty() { return; }
    let mut gained = 0.0f32;
    for ev in agent.digest.iter_mut() {
        if ev.remaining > 0 { gained += ev.per_step; ev.remaining -= 1; }
    }
    agent.digest.retain(|ev| ev.remaining > 0);
    agent.energy = (agent.energy + gained).min(MAX_ENERGY);
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
            let target_alive = agents[j].energy > 0.0 && agents[j].health > DEATH_HEALTH_THRESHOLD;
            // Newborn grace: attackers and targets in grace window ignore predation/scavenge resolution
            if agents[i].age_steps < NEWBORN_GRACE_STEPS { continue; }
            if agents[j].age_steps < NEWBORN_GRACE_STEPS { continue; }
            if (target_alive && !PREDATION_ENABLED) || (!target_alive && !SCAVENGE_ENABLED) { continue; }
            if target_alive {
                // Apply damage first. Respect brief invulnerability.
                if agents[j].invuln_steps == 0 {
                    agents[j].health -= PREDATION_DAMAGE;
                    agents[j].invuln_steps = INVULN_AFTER_HIT_STEPS;
                    agents[i].predation_flash_steps = agents[i].predation_flash_steps.saturating_add(8);
                    // Count a successful attack hit (damage applied)
                    agents[i].attack_hits = agents[i].attack_hits.saturating_add(1);
                }
                // If target died due to damage, convert to corpse and award meat later when scavenged/predated again.
                if agents[j].health <= DEATH_HEALTH_THRESHOLD {
                    // Count kill caused by this attacker
                    agents[i].kills_caused = agents[i].kills_caused.saturating_add(1);
                    agents[j].dead_since.get_or_insert(step_idx);
                    agents[j].corpse_energy = MEAT_ENERGY;
                    agents[j].energy = 0.0; // energy drained on death
                }
            } else {
                // Scavenge OR second predation hit on dead body -> consume corpse energy
                if agents[j].corpse_energy > 0.0 {
                    let gain = agents[j].corpse_energy.min(MEAT_ENERGY);
                    if DIGEST_STEPS_MEAT > 0 { agents[i].digest.push_back(DigestEvent { remaining: DIGEST_STEPS_MEAT, per_step: gain / (DIGEST_STEPS_MEAT as f32) }); }
                    else { agents[i].energy = (agents[i].energy + gain).min(MAX_ENERGY); }
                    // Healing bonus from meat
                    agents[i].health = (agents[i].health + MEAT_HEAL_BONUS).min(agents[i].max_health);
                    agents[i].eaten += 1;
                    agents[i].kills += 1; // counts scavenged meat as kill-equivalent for diet tint
                    agents[j].corpse_energy = 0.0;
                    agents[j].consumed = true;
                }
            }
            claimed[j] = true;
        }
    }
}

pub fn decay_corpses_and_flashes(agents: &mut [Agent]) {
    for a in agents.iter_mut() {
        if (a.energy <= 0.0 || a.health <= DEATH_HEALTH_THRESHOLD) && !a.consumed && a.corpse_energy > 0.0 {
            a.corpse_energy *= (1.0 - CORPSE_DECAY_RATE).max(0.0);
            if a.corpse_energy < 0.1 { a.corpse_energy = 0.0; a.consumed = true; }
        }
        if a.predation_flash_steps > 0 { a.predation_flash_steps -= 1; }
        if a.invuln_steps > 0 { a.invuln_steps -= 1; }
    }
}
