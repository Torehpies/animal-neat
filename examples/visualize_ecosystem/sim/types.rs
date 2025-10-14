use macroquad::prelude::Vec2;
use std::collections::VecDeque;
use crate::body::Body;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct AgentId(pub usize);

#[derive(Clone, Debug)]
pub struct Agent {
    pub id: AgentId,
    pub body: Body,
    pub theta: f32,
    pub energy: f32,
    pub health: f32,
    pub max_health: f32,
    pub invuln_steps: usize,
    pub alive_steps: u32,
    pub eaten: usize,
    pub consumed: bool, // true if this agent's body has been eaten and removed from world
    pub kills: usize,                // number of agents eaten (live or dead) in this episode
    pub predation_flash_steps: u16,  // visual cue counter for recent predation
    pub dead_since: Option<usize>,   // step index when this agent died
    pub corpse_energy: f32,          // remaining energy in corpse (for scavenging)
    pub digest: VecDeque<DigestEvent>, // incoming energy deliveries
    pub last_food_mem: Vec2,         // memory of last step's nearest-food local vector
    pub last_danger_mem: Vec2,       // memory of last step's nearest-agent local vector (legacy/general)
    pub last_same_mem: Vec2,         // memory of last step's nearest same-species local vector
    pub last_other_mem: Vec2,        // memory of last step's nearest other-species local vector
    pub species_id: usize,           // stable species index captured at episode start
    // Predation stats for fitness shaping
    pub attack_hits: usize,          // number of successful damage applications to live targets
    pub kills_caused: usize,         // number of times this agent's damage caused a death
    // (Removed) pooled sector proximities in revised vision model
    // Communication
    pub call_intensity: f32,      // emitted this step (0..1)
    pub heard_sectors: [f32;3],   // smoothed heard call energy (L,F,R)
    // Reproduction (eco continuous): cooldown and per-episode offspring count
    pub repro_cooldown: usize,
    pub offspring_count: usize,
}

#[derive(Clone, Copy, Debug)]
pub struct DigestEvent { 
    pub remaining: u16, 
    pub per_step: f32 
}

#[derive(Clone, Copy, Debug)]
pub struct CommSignal {
    pub caller: usize,       // agent index
    pub pos: Vec2,           // position of caller when signal created
    pub ttl: usize,          // remaining steps
}