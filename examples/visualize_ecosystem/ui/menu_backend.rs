//! Backend for the simulation configuration menu.
//!
//! Contains data structures and non-UI logic (parsing, clamping, applying values).

#[derive(Clone, Debug)]
pub struct SimConfig {
    pub world_width: f32,
    pub world_height: f32,
    pub population_size: usize,
    pub max_food: usize,
    pub food_respawn_prob: f32,
    pub initial_energy: f32,
    pub max_energy: f32,
    pub energy_drain_per_step: f32,
    // Fitness weights
    pub w_lifetime: f32,
    pub w_energy: f32,
    pub w_offspring: f32,
    pub w_comm: f32,
    pub w_idle_penalty: f32,
    pub w_plant: f32,
    pub w_meat: f32,
    pub w_attacks: f32,
    pub w_kills: f32,
    pub w_herding: f32,
    pub w_approach: f32,
    pub w_chase: f32,
    pub w_chase_same: f32,
}

impl Default for SimConfig {
    fn default() -> Self {
        Self {
            world_width: 750.0,
            world_height: 750.0,
            population_size: 50,
            max_food: 300,
            food_respawn_prob: 0.006,
            initial_energy: 500.0,
            max_energy: 5000.0,
            energy_drain_per_step: 0.05,
            w_lifetime: 0.05,
            w_energy: 5.0,
            w_offspring: 10.0,
            w_comm: 0.0,
            w_idle_penalty: 0.6,
            w_plant: 2.0,
            w_meat: 20.0,
            w_attacks: 10.0,
            w_kills: 20.0,
            w_herding: 0.5,
            w_approach: 0.1,
            w_chase: 0.3,
            w_chase_same: 0.4,
        }
    }
}

pub struct MenuState {
    pub config: SimConfig,
    pub editing_field: Option<EditField>,
    pub input_buffer: String,
    pub show_help: bool,
    pub help_scroll: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EditField {
    WorldWidth,
    WorldHeight,
    PopSize,
    MaxFood,
    FoodRespawnRate,
    InitialEnergy,
    MaxEnergy,
    EnergyDrain,
    WLifetime,
    WEnergy,
    WOffspring,
    WComm,
    WIdle,
    WPlant,
    WMeat,
    WAttacks,
    WKills,
    WHerd,
    WApproach,
    WChase,
    WChaseSame,
}

impl MenuState {
    pub fn new() -> Self {
        Self {
            config: SimConfig::default(),
            editing_field: None,
            input_buffer: String::new(),
            show_help: false,
            help_scroll: 0.0,
        }
    }
}

/// Apply a parsed numeric value to the given field, with appropriate clamping.
pub fn apply_field_value(cfg: &mut SimConfig, field: EditField, val: f32) {
    match field {
        EditField::WorldWidth => {
            cfg.world_width = val.max(100.0).min(2000.0);
        }
        EditField::WorldHeight => {
            cfg.world_height = val.max(100.0).min(2000.0);
        }
        EditField::PopSize => {
            // Allow much larger populations for stress-testing; clamp to 999,999
            cfg.population_size = (val as usize).max(1).min(999_999);
        }
        EditField::MaxFood => {
            // Allow a very large vegetation count; clamp to 999,999
            cfg.max_food = (val as usize).max(10).min(999_999);
        }
        EditField::FoodRespawnRate => {
            cfg.food_respawn_prob = val.max(0.0001).min(0.1);
        }
        EditField::InitialEnergy => {
            cfg.initial_energy = val.max(10.0).min(10000.0);
        }
        EditField::MaxEnergy => {
            cfg.max_energy = val.max(100.0).min(50000.0);
        }
        EditField::EnergyDrain => {
            cfg.energy_drain_per_step = val.max(0.0).min(10.0);
        }
        EditField::WLifetime => { cfg.w_lifetime = val; }
        EditField::WEnergy => { cfg.w_energy = val; }
        EditField::WOffspring => { cfg.w_offspring = val; }
        EditField::WComm => { cfg.w_comm = val; }
        EditField::WIdle => { cfg.w_idle_penalty = val.max(0.0); }
        EditField::WPlant => { cfg.w_plant = val; }
        EditField::WMeat => { cfg.w_meat = val; }
        EditField::WAttacks => { cfg.w_attacks = val; }
        EditField::WKills => { cfg.w_kills = val; }
        EditField::WHerd => { cfg.w_herding = val; }
        EditField::WApproach => { cfg.w_approach = val; }
        EditField::WChase => { cfg.w_chase = val; }
        EditField::WChaseSame => { cfg.w_chase_same = val; }
    }
}
