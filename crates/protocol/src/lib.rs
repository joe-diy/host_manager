//! NATS subject constants, OpenBao path constants, and message envelope types.
//!
//! All subjects and vault paths used across actors, providers, agent, and CLI
//! are defined here. **Never hardcode subjects or paths in actor/provider code.**

pub mod subjects;
pub mod vault_paths;
pub mod messages;

pub use subjects::*;
pub use vault_paths::*;
pub use messages::*;
