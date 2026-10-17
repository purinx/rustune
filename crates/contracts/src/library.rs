use crate::{InvalidInput, Patch, validate_number, validate_text};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrackSummary {
    pub id: Uuid,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration_ms: u64,
    pub artwork_id: Option<Uuid>,
    pub favorite: bool,
    pub revision: u64,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Source {
    pub root_name: String,
    pub relative_path: String,
    pub imported_at: String,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MetadataOrigin {
    Manual,
    Enriched,
    Imported,
    Fallback,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MetadataOrigins {
    pub title: MetadataOrigin,
    pub artist: MetadataOrigin,
    pub album: MetadataOrigin,
    pub album_artist: MetadataOrigin,
    pub disc_number: MetadataOrigin,
    pub track_number: MetadataOrigin,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrackDetail {
    pub summary: TrackSummary,
    pub album_artist: Option<String>,
    pub disc_number: Option<u16>,
    pub track_number: Option<u16>,
    pub sources: Vec<Source>,
    pub metadata_origins: MetadataOrigins,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MetadataPatch {
    pub expected_revision: u64,
    #[serde(default, skip_serializing_if = "Patch::is_keep")]
    pub title: Patch<String>,
    #[serde(default, skip_serializing_if = "Patch::is_keep")]
    pub artist: Patch<String>,
    #[serde(default, skip_serializing_if = "Patch::is_keep")]
    pub album: Patch<String>,
    #[serde(default, skip_serializing_if = "Patch::is_keep")]
    pub album_artist: Patch<String>,
    #[serde(default, skip_serializing_if = "Patch::is_keep")]
    pub disc_number: Patch<u16>,
    #[serde(default, skip_serializing_if = "Patch::is_keep")]
    pub track_number: Patch<u16>,
}
impl MetadataPatch {
    pub fn validate(mut self) -> Result<Self, InvalidInput> {
        for patch in [
            &mut self.title,
            &mut self.artist,
            &mut self.album,
            &mut self.album_artist,
        ] {
            if let Patch::Set(text) = patch {
                *text = validate_text(text, 512)?;
            }
        }
        for patch in [&self.disc_number, &self.track_number] {
            if let Patch::Set(n) = patch {
                validate_number(*n)?;
            }
        }
        Ok(self)
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlaylistItem {
    pub id: Uuid,
    pub track_id: Uuid,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Playlist {
    pub id: Uuid,
    pub name: String,
    pub revision: u64,
    pub items: Vec<PlaylistItem>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreatePlaylist {
    pub name: String,
}
impl CreatePlaylist {
    pub fn validate(mut self) -> Result<Self, InvalidInput> {
        self.name = validate_text(&self.name, 120)?;
        Ok(self)
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RenamePlaylist {
    pub expected_revision: u64,
    pub name: String,
}
impl RenamePlaylist {
    pub fn validate(mut self) -> Result<Self, InvalidInput> {
        self.name = validate_text(&self.name, 120)?;
        Ok(self)
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplacePlaylistItems {
    pub expected_revision: u64,
    pub items: Vec<PlaylistItem>,
}
impl ReplacePlaylistItems {
    pub fn validate(&self) -> Result<(), InvalidInput> {
        let mut ids = std::collections::HashSet::new();
        if self
            .items
            .iter()
            .any(|item| item.id.is_nil() || item.track_id.is_nil() || !ids.insert(item.id))
        {
            return Err(InvalidInput);
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AlbumSummary {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub artwork_id: Option<Uuid>,
    pub track_count: u64,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtistSummary {
    pub id: String,
    pub name: String,
    pub track_count: u64,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Exclusion {
    pub track_id: Uuid,
    pub title: String,
    pub excluded_at: String,
}
