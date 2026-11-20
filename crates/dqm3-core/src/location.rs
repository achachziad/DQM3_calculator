//! Where monsters can be recruited (`MonsterLocation.json`).

use crate::CoreError;
use crate::ids::{LocationId, MonsterId, SeasonId, WeatherId};
use crate::load::parse_table;
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

/// One place and condition where a monster appears in the wild.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Recruitment {
    pub location: LocationId,
    pub season: SeasonId,
    pub weather: WeatherId,
    /// Appears as a mini-boss there.
    pub mini_boss: bool,
}

/// The recruitments of a monster grouped by place: "at this location, in
/// these seasons, with these weathers". Lossless for the real data: for a
/// given place, every season × weather combination is present (checked in
/// docs/DATA.md §1.6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecruitmentPlace {
    pub location: LocationId,
    /// Sorted, without duplicates.
    pub seasons: Vec<SeasonId>,
    /// Sorted, without duplicates.
    pub weathers: Vec<WeatherId>,
    pub mini_boss: bool,
}

/// Groups recruitments by (location, mini-boss), sorted by location.
pub(crate) fn group_by_place(recruitments: &[Recruitment]) -> Vec<RecruitmentPlace> {
    // BTreeMap / BTreeSet keep their content sorted and free of
    // duplicates: grouping, sorting and dedup in one pass.
    let mut places: BTreeMap<(LocationId, bool), (BTreeSet<SeasonId>, BTreeSet<WeatherId>)> =
        BTreeMap::new();
    for r in recruitments {
        let (seasons, weathers) = places.entry((r.location, r.mini_boss)).or_default();
        seasons.insert(r.season);
        weathers.insert(r.weather);
    }
    places
        .into_iter()
        .map(
            |((location, mini_boss), (seasons, weathers))| RecruitmentPlace {
                location,
                seasons: seasons.into_iter().collect(),
                weathers: weathers.into_iter().collect(),
                mini_boss,
            },
        )
        .collect()
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

    fn at(location: u32, season: u32, weather: u32, mini_boss: bool) -> Recruitment {
        Recruitment {
            location: LocationId(location),
            season: SeasonId(season),
            weather: WeatherId(weather),
            mini_boss,
        }
    }

    #[test]
    fn recruitments_are_grouped_by_place() {
        let rows = [
            at(5, 2, 1, false),
            at(3, 4, 2, false),
            at(5, 1, 1, false),
            at(5, 2, 2, false),
            at(5, 1, 2, false),
            at(5, 3, 1, true),
        ];
        assert_eq!(
            group_by_place(&rows),
            vec![
                RecruitmentPlace {
                    location: LocationId(3),
                    seasons: vec![SeasonId(4)],
                    weathers: vec![WeatherId(2)],
                    mini_boss: false,
                },
                RecruitmentPlace {
                    location: LocationId(5),
                    seasons: vec![SeasonId(1), SeasonId(2)],
                    weathers: vec![WeatherId(1), WeatherId(2)],
                    mini_boss: false,
                },
                RecruitmentPlace {
                    location: LocationId(5),
                    seasons: vec![SeasonId(3)],
                    weathers: vec![WeatherId(1)],
                    mini_boss: true,
                },
            ]
        );
    }

    #[test]
    fn no_recruitment_gives_no_place() {
        assert!(group_by_place(&[]).is_empty());
    }

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
