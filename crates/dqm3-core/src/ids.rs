//! Identifier types.
//!
//! *Newtype* pattern: each id wraps a `u32` in its own type, so the compiler
//! refuses to mix them up (passing a `FamilyId` where a `MonsterId` is
//! expected is a compile error, not a silent bug). Zero runtime cost.

use serde::Deserialize;

/// `Copy`: small values copied implicitly (like a plain integer), no move.
/// `Hash` + `Eq`: usable as `HashMap` keys. `Ord`: sortable.
/// `#[serde(transparent)]`: read from JSON as the bare number inside.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Deserialize)]
#[serde(transparent)]
pub struct MonsterId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Deserialize)]
#[serde(transparent)]
pub struct FamilyId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Deserialize)]
#[serde(transparent)]
pub struct RankId(pub u32);
