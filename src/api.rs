//! Data structures for parsing DSS API responses.

pub use self::image::{ImageContent, ImageTile};
pub use self::text::{Language, Text, TextContent, TitleKind, Titles};

use fnv::FnvHashMap as HashMap;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

mod image;
mod text;

/// An API response containing home menu data.
#[derive(Debug, Serialize, Deserialize)]
pub struct Home {
    pub data: HashMap<String, Collection>,
}

/// An API response containing data for a curated set.
#[derive(Debug, Serialize, Deserialize)]
pub struct RefSet {
    pub data: HashMap<String, Set>,
}

/// A generic collection of menu data.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Collection {
    /// Indicates the collection kind and any custom fields.
    #[serde(flatten)]
    inner: CollectionInner,
    /// Image tiles to be displayed, keyed by name.
    #[serde(default)]
    image: HashMap<String, ImageTile>,
    /// Text data to be displayed.
    text: Text,
    /// Miniature video art for the collection, if any.
    #[serde(default)]
    video_art: Vec<VideoArt>,
}

impl Collection {
    /// Returns the kind of collection this is.
    pub fn kind(&self) -> CollectionKind {
        match self.inner {
            CollectionInner::DmcSeries { .. } => CollectionKind::DmcSeries,
            CollectionInner::DmcVideo { .. } => CollectionKind::DmcVideo,
            CollectionInner::StandardCollection { .. } => CollectionKind::Standard,
        }
    }

    /// Returns the elements within the collection, if any.
    ///
    /// Returns `Some` if this is a standard collection or `None` otherwise.
    pub fn containers(&self) -> Option<&[Container]> {
        match self.inner {
            CollectionInner::StandardCollection { ref containers, .. } => Some(containers),
            _ => None,
        }
    }

    /// Returns the associated image data to be displayed, if any, keyed by name.
    ///
    /// Standard collections _usually_ do not have images associated with them.
    pub fn images(&self) -> &HashMap<String, ImageTile> {
        &self.image
    }

    /// Returns the associated text data to be displayed, if any.
    pub fn text(&self) -> &Text {
        &self.text
    }
}

/// A list of valid collection types.
#[derive(Debug, PartialEq)]
pub enum CollectionKind {
    /// Indicates a series of videos, e.g. a television series.
    DmcSeries,
    /// Indicates a single video, e.g. a movie.
    DmcVideo,
    /// Contains several kinds of collections.
    Standard,
}

/// A list of special collection-specific fields.
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
enum CollectionInner {
    #[serde(rename_all = "camelCase")]
    DmcSeries {
        series_id: Uuid,
        encoded_series_id: String,
    },
    #[serde(rename_all = "camelCase")]
    DmcVideo { program_type: ProgramType },
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
pub struct Container {
    pub set: Set,
}

/// A set of menu items to display.
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Set {
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
pub struct Meta {
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
