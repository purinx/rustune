use unicode_normalization::UnicodeNormalization;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InvalidMetadata;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Text(String);
impl Text {
    pub fn new(value: &str) -> Result<Self, InvalidMetadata> {
        let value = value.trim();
        if !(1..=512).contains(&value.chars().count()) {
            return Err(InvalidMetadata);
        }
        Ok(Self(value.to_owned()))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TrackNumber(u16);
impl TrackNumber {
    pub fn new(value: u16) -> Result<Self, InvalidMetadata> {
        if (1..=999).contains(&value) {
            Ok(Self(value))
        } else {
            Err(InvalidMetadata)
        }
    }
    pub fn value(self) -> u16 {
        self.0
    }
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Metadata {
    pub title: Option<Text>,
    pub artist: Option<Text>,
    pub album: Option<Text>,
    pub album_artist: Option<Text>,
    pub disc_number: Option<TrackNumber>,
    pub track_number: Option<TrackNumber>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MetadataOrigin {
    Manual,
    Enriched,
    Imported,
    Fallback,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Resolved<T> {
    pub value: T,
    pub origin: MetadataOrigin,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EffectiveMetadata {
    pub title: Resolved<Text>,
    pub artist: Resolved<Text>,
    pub album: Resolved<Text>,
    pub album_artist: Resolved<Option<Text>>,
    pub disc_number: Resolved<Option<TrackNumber>>,
    pub track_number: Resolved<Option<TrackNumber>>,
}
fn choose<T: Clone>(
    imported: &Option<T>,
    enriched: &Option<T>,
    manual: &Option<T>,
) -> Resolved<Option<T>> {
    for (v, origin) in [
        (manual, MetadataOrigin::Manual),
        (enriched, MetadataOrigin::Enriched),
        (imported, MetadataOrigin::Imported),
    ] {
        if v.is_some() {
            return Resolved {
                value: v.clone(),
                origin,
            };
        }
    }
    Resolved {
        value: None,
        origin: MetadataOrigin::Fallback,
    }
}
fn text(i: &Option<Text>, e: &Option<Text>, m: &Option<Text>, fallback: &Text) -> Resolved<Text> {
    let r = choose(i, e, m);
    Resolved {
        value: r.value.unwrap_or_else(|| fallback.clone()),
        origin: r.origin,
    }
}
pub fn resolve_metadata(
    imported: &Metadata,
    enriched: &Metadata,
    manual: &Metadata,
    fallback: &Text,
) -> EffectiveMetadata {
    EffectiveMetadata {
        title: text(&imported.title, &enriched.title, &manual.title, fallback),
        artist: text(
            &imported.artist,
            &enriched.artist,
            &manual.artist,
            &Text::new("不明なアーティスト").expect("static valid text"),
        ),
        album: text(
            &imported.album,
            &enriched.album,
            &manual.album,
            &Text::new("不明なアルバム").expect("static valid text"),
        ),
        album_artist: choose(
            &imported.album_artist,
            &enriched.album_artist,
            &manual.album_artist,
        ),
        disc_number: choose(
            &imported.disc_number,
            &enriched.disc_number,
            &manual.disc_number,
        ),
        track_number: choose(
            &imported.track_number,
            &enriched.track_number,
            &manual.track_number,
        ),
    }
}
pub fn normalize_search(value: &str) -> String {
    value.nfkc().flat_map(char::to_lowercase).collect()
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum AlbumKey {
    Store(std::num::NonZeroU64),
    Metadata { album: String, artist: String },
}

/// アルバム・グループを手動編集すると、ストア由来の旧グループ化は解除される。
/// 呼び出し側は編集の適用時にこの解除を保存する。後から取り込み値へClearで戻す場合も同じ。
pub fn album_key(
    effective: &EffectiveMetadata,
    store_album_id: Option<std::num::NonZeroU64>,
    store_group_released: bool,
    compilation: bool,
) -> AlbumKey {
    let manually_grouped = effective.album.origin == MetadataOrigin::Manual
        || effective.album_artist.origin == MetadataOrigin::Manual;
    if !store_group_released
        && !manually_grouped
        && let Some(id) = store_album_id
    {
        return AlbumKey::Store(id);
    }
    let artist = effective
        .album_artist
        .value
        .as_ref()
        .map(Text::as_str)
        .unwrap_or_else(|| {
            if compilation {
                "Various Artists"
            } else {
                effective.artist.value.as_str()
            }
        });
    AlbumKey::Metadata {
        album: normalize_search(effective.album.value.as_str()),
        artist: normalize_search(artist),
    }
}
