use crate::track::{TrackId, TrackState};

pub const AUDIO_DIGEST_SCHEME: &str = "rustune-aac-probe-v1";
pub const PURCHASE_RULE: &str = "itunes-tag-pattern-v1";

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AudioDigest([u8; 32]);
impl AudioDigest {
    /// バージョン付き方式の接頭辞と、長さを前置したASC・AACサンプルのSHA-256。
    pub fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
    pub fn bytes(&self) -> &[u8; 32] {
        &self.0
    }
    pub fn scheme(&self) -> &'static str {
        AUDIO_DIGEST_SCHEME
    }
}

/// inspectionの境界が、この端末の購入タグ規則への一致を確認済み。タグ自体は編集できる。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct VerifiedStoreId(u32);
impl VerifiedStoreId {
    pub fn from_matched_tag_pattern(value: u32) -> Option<Self> {
        (value != 0).then_some(Self(value))
    }
    pub fn value(self) -> u32 {
        self.0
    }
    pub fn rule(self) -> &'static str {
        PURCHASE_RULE
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum IdentityKey {
    Store(VerifiedStoreId),
    Audio(AudioDigest),
}
pub fn resolve_identity(
    store_id: Option<VerifiedStoreId>,
    digest: AudioDigest,
) -> Vec<IdentityKey> {
    store_id
        .into_iter()
        .map(IdentityKey::Store)
        .chain([IdentityKey::Audio(digest)])
        .collect()
}

/// いずれかのキーで一致した既存行すべてと、保存済みの音声digest。
#[derive(Clone, Debug)]
pub struct IdentityMatch {
    pub track_id: TrackId,
    pub state: TrackState,
    pub digest: AudioDigest,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IdentityDecision {
    New,
    Existing(TrackId),
    Conflict,
    Excluded,
}
pub fn reconcile_identity(incoming: &AudioDigest, matches: &[IdentityMatch]) -> IdentityDecision {
    let Some(first) = matches.first() else {
        return IdentityDecision::New;
    };
    if matches
        .iter()
        .any(|m| m.track_id != first.track_id || &m.digest != incoming)
    {
        return IdentityDecision::Conflict;
    }
    if matches
        .iter()
        .any(|m| matches!(m.state, TrackState::Deleting | TrackState::Deleted))
    {
        return IdentityDecision::Excluded;
    }
    IdentityDecision::Existing(first.track_id)
}
