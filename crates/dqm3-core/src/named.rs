//! Small "id + English name" tables: sizes, locations, seasons, weathers.

use crate::CoreError;
use crate::ids::{LocationId, SeasonId, SizeId, WeatherId};
use crate::load::parse_table;
use serde::Deserialize;

/// An entry that is just an id and an English name.
///
/// Generic over the id type `I`: `Named<SeasonId>` and `Named<WeatherId>`
/// are different types, so they cannot be mixed up.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Named<I> {
    pub id: I,
    pub name_en: String,
}

/// Monster size: Small or Large (each monster has traits for both).
pub type Size = Named<SizeId>;
/// Recruitment area (35). No French names in the data.
pub type Location = Named<LocationId>;
/// Spring, Summer, Fall, Winter.
pub type Season = Named<SeasonId>;
/// Sun or Precipitation.
pub type Weather = Named<WeatherId>;

// The four files name their id field differently (`SizeId`, `LocationId`…),
// so each gets its own tiny raw row type.

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct RawSize {
    size_id: SizeId,
    name: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct RawLocation {
    location_id: LocationId,
    name: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct RawSeason {
    season_id: SeasonId,
    name: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct RawWeather {
    weather_id: WeatherId,
    name: String,
}

pub(crate) fn parse_sizes(json: &str) -> Result<Vec<Size>, CoreError> {
    let rows: Vec<RawSize> = parse_table("Size.json", json)?;
    Ok(rows
        .into_iter()
        .map(|r| Named {
            id: r.size_id,
            name_en: r.name,
        })
        .collect())
}

pub(crate) fn parse_locations(json: &str) -> Result<Vec<Location>, CoreError> {
    let rows: Vec<RawLocation> = parse_table("Location.json", json)?;
    Ok(rows
        .into_iter()
        .map(|r| Named {
            id: r.location_id,
            name_en: r.name,
        })
        .collect())
}

pub(crate) fn parse_seasons(json: &str) -> Result<Vec<Season>, CoreError> {
    let rows: Vec<RawSeason> = parse_table("Season.json", json)?;
    Ok(rows
        .into_iter()
        .map(|r| Named {
            id: r.season_id,
            name_en: r.name,
        })
        .collect())
}

pub(crate) fn parse_weathers(json: &str) -> Result<Vec<Weather>, CoreError> {
    let rows: Vec<RawWeather> = parse_table("Weather.json", json)?;
    Ok(rows
        .into_iter()
        .map(|r| Named {
            id: r.weather_id,
            name_en: r.name,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn small_tables_are_parsed() {
        let sizes = parse_sizes(
            r#"[{"SizeId": 2, "Name": "Large", "Identifier": "large", "ShortName": "L"}]"#,
        );
        assert_eq!(
            sizes.expect("valid"),
            vec![Named {
                id: SizeId(2),
                name_en: "Large".into()
            }]
        );

        let locations = parse_locations(
            r#"[{"LocationId": 31, "Name": "Aamon's Sanctum", "Identifier": "x", "JapaneseName": "y",
                 "MapLongitudeMax": null, "MapLatitudeMax": null, "MapLongitudeStart": null, "MapLatitudeStart": null}]"#,
        );
        assert_eq!(locations.expect("valid")[0].name_en, "Aamon's Sanctum");

        let seasons = parse_seasons(
            r#"[{"SeasonId": 3, "Name": "Fall", "Identifier": "fall", "JapaneseName": "秋"}]"#,
        );
        assert_eq!(seasons.expect("valid")[0].id, SeasonId(3));

        let weathers = parse_weathers(r#"[{"WeatherId": 1, "Name": "Sun", "Identifier": "sun"}]"#);
        assert_eq!(weathers.expect("valid")[0].name_en, "Sun");
    }

    #[test]
    fn invalid_small_table_names_its_file() {
        assert!(matches!(
            parse_weathers("{}"),
            Err(CoreError::Parse {
                file: "Weather.json",
                ..
            })
        ));
    }
}
