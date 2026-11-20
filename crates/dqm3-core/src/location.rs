//! Where monsters can be recruited (`MonsterLocation.json`).

use crate::CoreError;
use crate::ids::{LocationId, MonsterId, SeasonId, WeatherId};
use crate::load::parse_table;
use serde::Deserialize;

/// One place and condition where a monster appears in the wild.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Recruitment {
    pub location: LocationId,
    pub season: SeasonId,
    pub weather: WeatherId,
    /// Appears as a mini-boss there.
    pub mini_boss: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct RawMonsterLocation {
    monster_id: MonsterId,
    location_id: LocationId,
    season_id: SeasonId,
    weather_id: WeatherId,
    is_mini_boss: bool,
}

/// Parses `MonsterLocation.json` into `(monster, recruitment)` pairs.
pub(crate) fn parse_recruitments(json: &str) -> Result<Vec<(MonsterId, Recruitment)>, CoreError> {
    let rows: Vec<RawMonsterLocation> = parse_table("MonsterLocation.json", json)?;
    Ok(rows
        .into_iter()
        .map(|r| {
            let recruitment = Recruitment {
                location: r.location_id,
                season: r.season_id,
                weather: r.weather_id,
                mini_boss: r.is_mini_boss,
            };
            (r.monster_id, recruitment)
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recruitments_are_parsed() {
        let json = r#"[{"MonsterLocationId": 1, "MonsterId": 107, "LocationId": 3,
            "SeasonId": 2, "WeatherId": 1, "IsMiniBoss": true}]"#;
        assert_eq!(
            parse_recruitments(json).expect("valid"),
            vec![(
                MonsterId(107),
                Recruitment {
                    location: LocationId(3),
                    season: SeasonId(2),
                    weather: WeatherId(1),
                    mini_boss: true,
                }
            )]
        );
    }
}
