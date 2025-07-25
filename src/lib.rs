pub mod circuit;
pub mod cli;
pub mod mna;
pub mod output;
pub mod parser;
pub mod simulator;
pub mod solver;

// Re-export commonly used types
pub use circuit::{Circuit, Component, Node};
pub use parser::SpiceParser;
pub use simulator::{Simulator, SimulationResult};

// Error types
pub type Result<T> = anyhow::Result<T>;

// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION"); 