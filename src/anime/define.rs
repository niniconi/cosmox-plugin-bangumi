use serde::{Deserialize, Serialize};

use cosmox_api::extend::MetadataExtend;

/// Bangumi subject id, written into the series node's extend map when the
/// network scrape resolves a series to a Bangumi subject. The base anime
/// plugin's shared `Anime` struct intentionally carries no id — ids are
/// source-specific and owned by the scraping plugin.
#[derive(Debug, Default, Serialize, Deserialize, MetadataExtend)]
#[extend(key = "anime-bangumi")]
pub struct BangumiAnime {
    pub id: u64,
}

/// Bangumi subject id of a season, written into the `Season N` node's
/// extend map when the network scrape resolves it. The base anime plugin's
/// shared `Season` struct carries no id for the same reason as `Anime`.
#[derive(Debug, Default, Serialize, Deserialize, MetadataExtend)]
#[extend(key = "anime-bangumi-season")]
pub struct BangumiSeason {
    pub id: u64,
}

/// Bangumi character id, written into each character node created by
/// [`crate::anime::action::enrich::enrich_cast`].
#[derive(Debug, Default, Serialize, Deserialize, MetadataExtend)]
#[extend(key = "anime-bangumi-character")]
pub struct BangumiCharacter {
    pub id: u64,
}

/// Bangumi person id, written into each staff node created by
/// [`crate::anime::action::enrich::enrich_staff`].
#[derive(Debug, Default, Serialize, Deserialize, MetadataExtend)]
#[extend(key = "anime-bangumi-staff")]
pub struct BangumiStaff {
    pub id: u64,
}
