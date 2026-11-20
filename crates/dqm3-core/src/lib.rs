//! Core of the DQM3 synthesis tool: data models, loading, indexes and
//! queries. No UI, no network: the GUI and the MCP server are thin
//! adapters over this library.

mod database;
mod error;
mod family;
mod ids;
mod index;
mod load;
mod location;
mod monster;
mod named;
mod recipe;
mod search;
mod synthesis;
mod talent;
mod traits;

pub use database::Database;
pub use error::CoreError;
pub use family::{Family, Rank};
pub use ids::{FamilyId, MonsterId, RankId};
pub use ids::{LocationId, SeasonId, SizeId, SkillId, TalentId, TraitId, WeatherId};
pub use location::Recruitment;
pub use monster::{Monster, Stats};
pub use named::{Location, Named, Season, Size, Weather};
pub use recipe::{Parents, Recipe};
pub use search::{MonsterQuery, SortKey, Stat, fold};
pub use talent::{MonsterTalent, Reward, Skill, Talent, TalentEvolution, TalentTier};
pub use traits::{MonsterTrait, Trait};
