//! Backend for the simulation configuration menu.
//!
//! Contains data structures and non-UI logic (parsing, clamping, applying values).

#[derive(Clone, Debug)]
pub struct SimConfig {
    pub world_width: f32,
    pub world_height: f32,
    pub population_size: usize,
    // split population into two ecological groups
    pub herbivore_count: usize,
    pub carnivore_count: usize,
    pub max_food: usize,
    pub food_respawn_prob: f32,
    // Per-kind energy configuration
    pub initial_energy_herb: f32,
    pub max_energy_herb: f32,
    pub energy_drain_per_step_herb: f32,
    pub initial_energy_carn: f32,
    pub max_energy_carn: f32,
    pub energy_drain_per_step_carn: f32,
    // Fitness weights (per AgentKind)
    // Herbivore-specific
    pub w_lifetime_herb: f32,
    pub w_energy_herb: f32,
    pub w_offspring_herb: f32,
    pub w_comm_herb: f32,
    pub w_idle_penalty_herb: f32,
    pub w_plant_herb: f32,
    pub w_meat_herb: f32,
    pub w_attacks_herb: f32,
    pub w_kills_herb: f32,
    pub w_herding_herb: f32,
    // Carnivore-specific
    pub w_lifetime_carn: f32,
    pub w_energy_carn: f32,
    pub w_offspring_carn: f32,
    pub w_comm_carn: f32,
    pub w_idle_penalty_carn: f32,
    pub w_plant_carn: f32,
    pub w_meat_carn: f32,
    pub w_attacks_carn: f32,
    pub w_kills_carn: f32,
    pub w_herding_carn: f32,
    // Behavior shaping per kind
    pub w_approach_herb: f32,
    pub w_chase_herb: f32,
    pub w_chase_same_herb: f32,
    pub w_approach_carn: f32,
    pub w_chase_carn: f32,
    pub w_chase_same_carn: f32,
    // Offspring reproduction parameters (per kind)
    pub offspring_energy_herb: f32,
    pub offspring_energy_carn: f32,
    pub offspring_cooldown_herb: usize,
    pub offspring_cooldown_carn: usize,
}

impl Default for SimConfig {
    fn default() -> Self {
        Self {
            world_width: 750.0,
            world_height: 750.0,
            // default split: half herbivores, half carnivores
            herbivore_count: 25,
            carnivore_count: 25,
            population_size: 25 + 25,
            max_food: 300,
            food_respawn_prob: 0.006,
            initial_energy_herb: 500.0,
            max_energy_herb: 5000.0,
            energy_drain_per_step_herb: 0.05,
            initial_energy_carn: 500.0,
            max_energy_carn: 5000.0,
            energy_drain_per_step_carn: 0.05,
            // Defaults: herbivores flee and eat plants; carnivores hunt and eat meat
            // Herbivores: no meat/attacks/kills/chase (they flee instead)
            w_lifetime_herb: 0.05,
            w_energy_herb: 5.0,
            w_offspring_herb: 10.0,
            w_comm_herb: 0.0,
            w_idle_penalty_herb: 0.6,
            w_plant_herb: 2.0,
            w_meat_herb: 0.0,        // Herbivores don't eat meat
            w_attacks_herb: 0.0,     // Herbivores don't attack
            w_kills_herb: 0.0,       // Herbivores don't kill
            w_herding_herb: 0.5,
            // Carnivores: no plants (they hunt meat only)
            w_lifetime_carn: 0.05,
            w_energy_carn: 5.0,
            w_offspring_carn: 10.0,
            w_comm_carn: 0.0,
            w_idle_penalty_carn: 0.6,
            w_plant_carn: 0.0,       // Carnivores don't eat plants
            w_meat_carn: 20.0,
            w_attacks_carn: 10.0,
            w_kills_carn: 20.0,
            w_herding_carn: 0.5,
            // Behavior shaping: herbivores flee (not chase), carnivores hunt (chase other species)
            w_approach_herb: 0.0,    // Herbivores don't approach meat/carnivores
            w_chase_herb: 0.0,       // Herbivores don't chase (they flee instead)
            w_chase_same_herb: 0.4,  // Herbivores can still follow their own kind (herding)
            w_approach_carn: 0.0,    // Carnivores don't approach plants
            w_chase_carn: 0.3,       // Carnivores chase herbivores (hunt)
            w_chase_same_carn: 0.4,  // Carnivores can coordinate with pack
            // Offspring reproduction: herbivores reproduce faster, carnivores need longer cooldown
            offspring_energy_herb: 250.0,
            offspring_energy_carn: 250.0,
            offspring_cooldown_herb: 50,
            offspring_cooldown_carn: 100, // Carnivores have longer reproduction cooldown
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuScreen {
    Core,
    Fitness,
}

pub struct MenuState {
    pub config: SimConfig,
    pub editing_field: Option<EditField>,
    pub input_buffer: String,
    pub show_help: bool,
    pub help_scroll: f32,
    pub screen: MenuScreen,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EditField {
    WorldWidth,
    WorldHeight,
    PopSize,
    Herbivores,
    Carnivores,
    MaxFood,
    FoodRespawnRate,
    // Per-kind energy fields
    InitialEnergyHerb,
    MaxEnergyHerb,
    EnergyDrainHerb,
    InitialEnergyCarn,
    MaxEnergyCarn,
    EnergyDrainCarn,
    // Herbivore weights
    WLifetimeHerb,
    WEnergyHerb,
    WOffspringHerb,
    WCommHerb,
    WIdleHerb,
    WPlantHerb,
    WMeatHerb,
    WAttacksHerb,
    WKillsHerb,
    WHerdHerb,
    // Carnivore weights
    WLifetimeCarn,
    WEnergyCarn,
    WOffspringCarn,
    WCommCarn,
    WIdleCarn,
    WPlantCarn,
    WMeatCarn,
    WAttacksCarn,
    WKillsCarn,
    WHerdCarn,
    // Behavior shaping per kind
    WApproachHerb,
    WChaseHerb,
    WChaseSameHerb,
    WApproachCarn,
    WChaseCarn,
    WChaseSameCarn,
}

impl MenuState {
    pub fn new() -> Self {
        Self {
            config: SimConfig::default(),
            editing_field: None,
            input_buffer: String::new(),
            show_help: false,
            help_scroll: 0.0,
            screen: MenuScreen::Core,
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
            // if explicit species counts are zero, split evenly; otherwise preserve proportions by scaling
            let total_spec = cfg.herbivore_count + cfg.carnivore_count;
            if total_spec == 0 {
                let half = cfg.population_size / 2;
                cfg.herbivore_count = half;
                cfg.carnivore_count = cfg.population_size - half;
            } else {
                // scale existing counts to match new total (preserve ratio)
                let h = cfg.herbivore_count as f32;
                let c = cfg.carnivore_count as f32;
                let sum = (h + c).max(1.0);
                cfg.herbivore_count = ((h / sum) * (cfg.population_size as f32)).round() as usize;
                cfg.carnivore_count = cfg.population_size.saturating_sub(cfg.herbivore_count);
            }
        }
        EditField::Herbivores => {
            cfg.herbivore_count = (val as usize).max(0).min(999_999);
            cfg.population_size = cfg.herbivore_count + cfg.carnivore_count;
        }
        EditField::Carnivores => {
            cfg.carnivore_count = (val as usize).max(0).min(999_999);
            cfg.population_size = cfg.herbivore_count + cfg.carnivore_count;
        }
        EditField::MaxFood => {
            // Allow a very large vegetation count; clamp to 999,999
            cfg.max_food = (val as usize).max(10).min(999_999);
        }
        EditField::FoodRespawnRate => {
            cfg.food_respawn_prob = val.max(0.0001).min(0.1);
        }
        EditField::InitialEnergyHerb => { cfg.initial_energy_herb = val.max(10.0).min(10000.0); }
        EditField::MaxEnergyHerb => { cfg.max_energy_herb = val.max(100.0).min(50000.0); }
        EditField::EnergyDrainHerb => { cfg.energy_drain_per_step_herb = val.max(0.0).min(10.0); }
        EditField::InitialEnergyCarn => { cfg.initial_energy_carn = val.max(10.0).min(10000.0); }
        EditField::MaxEnergyCarn => { cfg.max_energy_carn = val.max(100.0).min(50000.0); }
        EditField::EnergyDrainCarn => { cfg.energy_drain_per_step_carn = val.max(0.0).min(10.0); }
        // Herbivore weights
        EditField::WLifetimeHerb => { cfg.w_lifetime_herb = val; }
        EditField::WEnergyHerb => { cfg.w_energy_herb = val; }
        EditField::WOffspringHerb => { cfg.w_offspring_herb = val; }
        EditField::WCommHerb => { cfg.w_comm_herb = val; }
        EditField::WIdleHerb => { cfg.w_idle_penalty_herb = val.max(0.0); }
        EditField::WPlantHerb => { cfg.w_plant_herb = val; }
        EditField::WMeatHerb => { cfg.w_meat_herb = val; }
        EditField::WAttacksHerb => { cfg.w_attacks_herb = val; }
        EditField::WKillsHerb => { cfg.w_kills_herb = val; }
        EditField::WHerdHerb => { cfg.w_herding_herb = val; }
        // Carnivore weights
        EditField::WLifetimeCarn => { cfg.w_lifetime_carn = val; }
        EditField::WEnergyCarn => { cfg.w_energy_carn = val; }
        EditField::WOffspringCarn => { cfg.w_offspring_carn = val; }
        EditField::WCommCarn => { cfg.w_comm_carn = val; }
        EditField::WIdleCarn => { cfg.w_idle_penalty_carn = val.max(0.0); }
        EditField::WPlantCarn => { cfg.w_plant_carn = val; }
        EditField::WMeatCarn => { cfg.w_meat_carn = val; }
        EditField::WAttacksCarn => { cfg.w_attacks_carn = val; }
        EditField::WKillsCarn => { cfg.w_kills_carn = val; }
        EditField::WHerdCarn => { cfg.w_herding_carn = val; }
        // Behavior shaping per kind
        EditField::WApproachHerb => { cfg.w_approach_herb = val; }
        EditField::WChaseHerb => { cfg.w_chase_herb = val; }
        EditField::WChaseSameHerb => { cfg.w_chase_same_herb = val; }
        EditField::WApproachCarn => { cfg.w_approach_carn = val; }
        EditField::WChaseCarn => { cfg.w_chase_carn = val; }
        EditField::WChaseSameCarn => { cfg.w_chase_same_carn = val; }
    }
}
