//! NEAT (NeuroEvolution of Augmenting Topologies) library
//!
//! This crate provides a small, self-contained implementation of NEAT that you can embed in your
//! applications. The public API re-exports the core modules so you can directly use items like
//! `runner::run_neat_xor`, `genome::Genome`, etc.
//!
//! Quick start:
//!
//! ```no_run
//! // Evolve a solution to XOR.
//! neat::runner::run_neat_xor(
//!     true,   // save best genome
//!     1000,   // generations
//!     50,     // population size
//!     3.9,    // target fitness
//!     2.0,    // speciator threshold
//! );
//! ```
//!
//! For more control, construct and evolve populations yourself using the public modules.

// Keep the existing module tree under `neat` for internal organization.
pub mod neat;

// Re-export the inner modules at the crate root for ergonomic access, allowing `neat::runner` etc.
pub use neat::*;
