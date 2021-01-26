//! Types for deserializing text data.

use fnv::FnvHashMap as HashMap;
use serde::{Deserialize, Serialize};

/// Node containing text data.
#[derive(Debug, Serialize, Deserialize)]
pub struct Text {
    /// Contains title text data.
    pub title: Titles,
}

/// A list of valid title types.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TitleKind {
    /// Full title to be displayed to the user.
    Full,
    /// Slug title to be consumed by business logic.
    Slug,
}

/// Contains the text for one or more titles.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename = "lowercase")]
pub struct Titles(HashMap<TitleKind, Kind>);

impl Titles {
    /// Returns the text content for the given title.
    pub fn get(&self, kind: TitleKind) -> Option<&TextContent> {
        self.0.get(&kind).map(|kind| match kind {
            Kind::Collection { default } => default,
            Kind::Program { default } => default,
            Kind::Series { default } => default,
            Kind::Set { default } => default,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Kind {
    Collection { default: TextContent },
    Program { default: TextContent },
    Series { default: TextContent },
    Set { default: TextContent },
}

/// Contains the text content and metadata
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextContent {
    /// Text to display.
    pub content: String,
    /// Localization of the text.
    pub language: Language,
}

/// A list of supported natural language localizations.
#[derive(Debug, Serialize, Deserialize)]
pub enum Language {
    /// English (US)
    #[serde(rename = "en")]
    English,
}
