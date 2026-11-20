//! Identifier types.
//!
//! *Newtype* pattern: each id wraps a `u32` in its own type, so the compiler
//! refuses to mix them up (passing a `FamilyId` where a `MonsterId` is
//! expected is a compile error, not a silent bug). Zero runtime cost.

/// Defines one id type per name, all with the same traits:
/// - `Copy`: copied implicitly like a plain integer (no move);
/// - `Hash` + `Eq`: usable as `HashMap` keys; `Ord`: sortable;
/// - `Deserialize` with `transparent`: read from JSON as the bare number;
/// - `Display`: shown as the bare number.
///
/// `macro_rules!` generates code at compile time: one definition here
/// instead of ten copy-pasted blocks.
macro_rules! id_types {
    ($($(#[$doc:meta])* $name:ident),* $(,)?) => {
        $(
            $(#[$doc])*
            #[derive(
                Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord,
                serde::Deserialize,
            )]
            #[serde(transparent)]
            pub struct $name(pub u32);

            impl std::fmt::Display for $name {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    write!(f, "{}", self.0)
                }
            }
        )*
    };
}

id_types!(
    MonsterId, FamilyId, RankId, TalentId, SkillId, TraitId, SizeId, LocationId, SeasonId,
    WeatherId,
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_display_as_their_number() {
        assert_eq!(MonsterId(42).to_string(), "42");
        assert_eq!(TalentId(7).to_string(), "7");
    }

    #[test]
    fn ids_are_read_from_json_as_bare_numbers() {
        let id: SkillId = serde_json::from_str("12").expect("valid json");
        assert_eq!(id, SkillId(12));
    }
}
