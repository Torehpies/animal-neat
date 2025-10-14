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
    if gained > 0.0 {
        // digestion now increases internal satiety reserve rather than directly topping up energy
        // The actual energy available to the agent will be supplied gradually by converting satiety
        // into energy in the engine tick. Increase satiety proportional to the energy-equivalent
        // that digestion delivers this tick.
        agent.satiety = (agent.satiety + gained * SATIETY_GAIN_PER_ENERGY_DELIVERED).clamp(0.0, 1.0);
    }
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
            if (target_alive && !PREDATION_ENABLED) || (!target_alive && !SCAVENGE_ENABLED) { continue; }
            if target_alive {
                // Apply damage first. Respect brief invulnerability.
                if agents[j].invuln_steps == 0 {
                    // Calculate damage based on attacker's diet
                    let total_intake = agents[i].eaten as f32;
                    let meat_intake = agents[i].kills as f32;
                    let damage_multiplier = if total_intake > 0.0 {
                        let meat_ratio = meat_intake / total_intake;
                        if meat_ratio < DIET_HERBIVORE_THRESHOLD {
                            HERBIVORE_DAMAGE_MULTIPLIER  // weak attacker
                        } else if meat_ratio > DIET_CARNIVORE_THRESHOLD {
                            CARNIVORE_DAMAGE_MULTIPLIER  // strong attacker
                        } else {
                            OMNIVORE_DAMAGE_MULTIPLIER   // moderate attacker
                        }
                    } else {
                        // No intake history yet - default to omnivore multiplier
                        OMNIVORE_DAMAGE_MULTIPLIER
                    };
                    let damage = PREDATION_DAMAGE * damage_multiplier;
                    agents[j].health -= damage;
                    agents[j].invuln_steps = INVULN_AFTER_HIT_STEPS;
                    agents[i].predation_flash_steps = agents[i].predation_flash_steps.saturating_add(8);
                }
                // If target died due to damage, convert to corpse and award meat later when scavenged/predated again.
                if agents[j].health <= DEATH_HEALTH_THRESHOLD {
                    agents[j].dead_since.get_or_insert(step_idx);
                    agents[j].corpse_energy = MEAT_ENERGY;
                    agents[j].energy = 0.0; // energy drained on death
                }
            } else {
                // Scavenge OR second predation hit on dead body -> consume corpse energy
                if agents[j].corpse_energy > 0.0 {
                    let gain = agents[j].corpse_energy.min(MEAT_ENERGY);
                    if DIGEST_STEPS_MEAT > 0 {
                        // deliver via digestion which will increase satiety over time
                        agents[i].digest.push_back(DigestEvent { remaining: DIGEST_STEPS_MEAT, per_step: gain / (DIGEST_STEPS_MEAT as f32) });
                    } else {
                        // instant delivery: convert energy-equivalent straight into satiety reserve
                        agents[i].satiety = (agents[i].satiety + gain * SATIETY_GAIN_PER_ENERGY_DELIVERED).clamp(0.0, 1.0);
                    }
                    // Healing bonus from meat
                    agents[i].health = (agents[i].health + MEAT_HEAL_BONUS).min(agents[i].max_health);
                    // Increase satiety: if energy was applied immediately, bump now, otherwise digestion will increase satiety gradually
                    if DIGEST_STEPS_MEAT == 0 {
                        agents[i].satiety = (agents[i].satiety + SATIETY_GAIN_FROM_MEAT).clamp(0.0, 1.0);
                    }
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
