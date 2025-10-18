use serde::{Deserialize, Serialize};
use std::io;

use crate::sim::{Agent, AgentKind};
use macroquad::prelude::Vec2;
use neat::neat::{genome::Genome, innovation_tracker::InnovationTracker};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Vec2Ser {
    pub x: f32,
    pub y: f32,
}

impl From<Vec2> for Vec2Ser {
    fn from(v: Vec2) -> Self {
        Vec2Ser { x: v.x, y: v.y }
    }
}

impl Vec2Ser {
    pub fn to_vec2(&self) -> Vec2 {
        Vec2 { x: self.x, y: self.y }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgentSnapshot {
    pub kind: AgentKind,
    pub body_pos: Vec2Ser,
    pub body_vel: Vec2Ser,
    pub theta: f32,
    pub energy: f32,
    pub health: f32,
    pub max_health: f32,
    pub invuln_steps: usize,
    pub alive_steps: u32,
    pub eaten: usize,
    pub consumed: bool,
    pub kills: usize,
    pub predation_flash_steps: u16,
    pub dead_since: Option<usize>,
    pub corpse_energy: f32,
    // digest is transient; omit and reinit empty on load
    pub last_food_mem: Vec2Ser,
    pub last_danger_mem: Vec2Ser,
    pub last_same_mem: Vec2Ser,
    pub last_other_mem: Vec2Ser,
    pub species_id: usize,
    pub age_steps: usize,
    pub call_intensity: f32,
    pub heard_sectors: [f32; 3],
    pub repro_cooldown: usize,
    pub offspring_count: usize,
    pub attack_hits: usize,
    pub kills_caused: usize,
    pub idle_anchor: Vec2Ser,
    pub idle_steps: usize,
    pub total_idle_penalty: f32,
}

impl From<&Agent> for AgentSnapshot {
    fn from(a: &Agent) -> Self {
        AgentSnapshot {
            kind: a.kind,
            body_pos: Vec2Ser::from(a.body.pos),
            body_vel: Vec2Ser::from(a.body.vel),
            theta: a.theta,
            energy: a.energy,
            health: a.health,
            max_health: a.max_health,
            invuln_steps: a.invuln_steps,
            alive_steps: a.alive_steps,
            eaten: a.eaten,
            consumed: a.consumed,
            kills: a.kills,
            predation_flash_steps: a.predation_flash_steps,
            dead_since: a.dead_since,
            corpse_energy: a.corpse_energy,
            last_food_mem: Vec2Ser::from(a.last_food_mem),
            last_danger_mem: Vec2Ser::from(a.last_danger_mem),
            last_same_mem: Vec2Ser::from(a.last_same_mem),
            last_other_mem: Vec2Ser::from(a.last_other_mem),
            species_id: a.species_id,
            age_steps: a.age_steps,
            call_intensity: a.call_intensity,
            heard_sectors: a.heard_sectors,
            repro_cooldown: a.repro_cooldown,
            offspring_count: a.offspring_count,
            attack_hits: a.attack_hits,
            kills_caused: a.kills_caused,
            idle_anchor: Vec2Ser::from(a.idle_anchor),
            idle_steps: a.idle_steps,
            total_idle_penalty: a.total_idle_penalty,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SimSnapshot {
    pub generation: usize,
    pub population: Vec<Genome>,
    pub innovation: InnovationTracker,
    pub food: Vec<Vec2Ser>,
    pub agents: Vec<AgentSnapshot>,
    pub member_species: Vec<usize>,
    pub episode_steps: usize,
}

/// Save the full simulation snapshot (population + innovation + world + episode agents)
pub fn save_sim_snapshot(
    path: &str,
    generation: usize,
    population: &[Genome],
    innov: &InnovationTracker,
    episode: &crate::sim::Episode,
    member_species: &[usize],
) -> io::Result<()> {
    let snap = SimSnapshot {
        generation,
        population: population.to_vec(),
        innovation: innov.clone(),
        food: episode.food.iter().map(|p| Vec2Ser::from(*p)).collect(),
        agents: episode.agents.iter().map(|a| AgentSnapshot::from(a)).collect(),
        member_species: member_species.to_vec(),
        episode_steps: episode.steps,
    };
    let s = serde_json::to_string_pretty(&snap).map_err(io::Error::other)?;
    if let Some(parent) = std::path::Path::new(path).parent() {
        if !parent.as_os_str().is_empty() { let _ = std::fs::create_dir_all(parent); }
    }
    std::fs::write(path, s)
}

/// Load a previously saved SimSnapshot
pub fn load_sim_snapshot(path: &str) -> io::Result<SimSnapshot> {
    let s = std::fs::read_to_string(path)?;
    let snap: SimSnapshot = serde_json::from_str(&s).map_err(io::Error::other)?;
    Ok(snap)
}
