//! Core of the DQM3 synthesis tool: data models, loading, indexes and
//! queries. No UI, no network: the GUI and the MCP server are thin
//! adapters over this library.

mod database;
mod error;
mod ids;
mod load;
mod monster;

pub use database::Database;
pub use error::CoreError;
pub use ids::{FamilyId, MonsterId, RankId};
pub use monster::{Monster, Stats};
