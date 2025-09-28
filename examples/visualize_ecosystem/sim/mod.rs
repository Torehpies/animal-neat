pub mod episode;
pub mod sim;
pub mod types;
pub mod util;
pub mod eval;
pub mod engine;

pub use episode::Episode;
pub use types::{Agent, AgentId, DigestEvent, CommSignal};
pub use util::{dir_from_theta, grid_index, mask_inputs};
pub use eval::{eval_population_single_episode, build_species_map};
pub use engine::{tick_step, StepDelta};
pub use sim::{apply_digestion, resolve_predation, decay_corpses_and_flashes};