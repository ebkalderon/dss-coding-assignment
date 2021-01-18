//! Data structures for image tiles.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// An image tile scaled to fit several aspect ratios.
#[derive(Debug, Serialize, Deserialize)]
pub struct ImageTile(HashMap<String, Kind>);

impl ImageTile {
    /// Returns the image content scaled to the given aspect ratio.
    pub fn get(&self, aspect_ratio: &str) -> Option<&ImageContent> {
        self.0.get(aspect_ratio).map(|kind| match kind {
            Kind::Default { default } => default,
            Kind::Program { default } => default,
            Kind::Series { default } => default,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Kind {
    Default { default: ImageContent },
    Program { default: ImageContent },
    Series { default: ImageContent },
}

/// A retrievable JPEG image.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageContent {
    /// Maximum height of the full-resolution image.
    pub master_height: u32,
    /// Maximum width of the full-resolution image.
    pub master_width: u32,
    /// Source URL where the image file can be retrieved.
    ///
    /// The image resolution is usually scaled down by default. Different image sizes can be
    /// requested by adjusting the query parameters relative to the `master_height` and
    /// `master_width` fields, respectively.
    pub url: String,
}
