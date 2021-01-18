//! Data structures for parsing DSS API responses.

pub use self::image::{ImageContent, ImageTile};
pub use self::text::{Language, Text, TextContent, Titles};

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

mod image;
mod text;

/// An API response containing home menu data.
#[derive(Debug, Serialize, Deserialize)]
pub struct Home {
    data: BTreeMap<String, Collection>,
}

/// An API response containing data for a curated set.
#[derive(Debug, Serialize, Deserialize)]
pub struct RefSet {
    data: BTreeMap<String, Set>,
}

/// A generic collection of menu data.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Collection {
    /// Indicates the collection kind and any custom fields.
    #[serde(flatten)]
    kind: CollectionKind,
    /// Image tiles to be displayed, keyed by name.
    #[serde(default)]
    image: BTreeMap<String, ImageTile>,
    /// Text data to be displayed.
    text: Text,
    /// Miniature video art for the collection, if any.
    #[serde(default)]
    video_art: Vec<VideoArt>,
}

/// A list of valid menu collection types.
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
enum CollectionKind {
    /// Indicates a series of videos, e.g. a television series.
    #[serde(rename_all = "camelCase")]
    DmcSeries {
        series_id: Uuid,
        encoded_series_id: String,
    },
    /// Indicates a single video, e.g. a movie.
    #[serde(rename_all = "camelCase")]
    DmcVideo { program_type: ProgramType },
    /// Contains several collections.
    #[serde(rename_all = "camelCase")]
    StandardCollection {
        /// Unique ID of the standard collection.
        collection_id: Uuid,
        #[serde(default)]
        containers: Vec<Container>,
    },
}

/// A list of video programming types.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum ProgramType {
    /// Indicates a standard film.
    Movie,
    /// Indicates a short-form video.
    ShortForm,
}

/// A menu container containing a set of items.
#[derive(Debug, Serialize, Deserialize)]
struct Container {
    set: Set,
}

/// A set of menu items to display.
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
enum Set {
    /// A curated set of menu items.
    #[serde(alias = "PersonalizedCuratedSet")]
    CuratedSet {
        items: Vec<Collection>,
        meta: Meta,
        text: Text,
    },
    /// A remote set that must be fetched over the network.
    SetRef {
        #[serde(rename = "refId")]
        ref_id: Uuid,
        text: Text,
    },
}

/// Contains metadata for a curated set.
#[derive(Debug, Serialize, Deserialize)]
struct Meta {
    hits: u32,
    offset: u16,
    page_size: u32,
}

/// Contains background video art data.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct VideoArt {
    media_metadata: MediaMetadata,
}

/// Contains a list of background video URLs.
#[derive(Debug, Serialize, Deserialize)]
struct MediaMetadata {
    urls: Vec<Url>,
}

/// A downloadable URL for a video file.
#[derive(Debug, Serialize, Deserialize)]
struct Url {
    url: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_home_json() {
        let json = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/home.json"));
        let _: Home = serde_json::from_str(json).expect("failed to parse `home.json`");
    }

    #[test]
    fn parses_ref_set() {
        let json = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/f506622c-4f75-4f87-bafe-3e08a4433914.json"
        ));
        let _: RefSet = serde_json::from_str(json).expect("failed to parse \"ref\" set JSON");
    }
}
