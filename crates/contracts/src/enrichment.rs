use crate::{InvalidInput, validate_text};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SearchRequest {
    pub query: String,
}
impl SearchRequest {
    pub fn validate(mut self) -> Result<Self, InvalidInput> {
        self.query = validate_text(&self.query, 512)?;
        Ok(self)
    }
}
/// No preview URL or arbitrary image URL is accepted back from the browser.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Candidate {
    pub id: Uuid,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub store_url: String,
    pub artwork_available: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnrichmentField {
    Title,
    Artist,
    Album,
    AlbumArtist,
    DiscNumber,
    TrackNumber,
    Artwork,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApplyRequest {
    pub track_id: Uuid,
    pub candidate_id: Uuid,
    pub expected_revision: u64,
    pub fields: Vec<EnrichmentField>,
    pub clear_manual_fields: Vec<EnrichmentField>,
}
