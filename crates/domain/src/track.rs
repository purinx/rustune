use crate::identity::{AudioDigest, VerifiedStoreId};
use uuid::Uuid;

macro_rules! id {
    ($name:ident) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(Uuid);
        impl $name {
            pub fn new(value: Uuid) -> Result<Self, InvalidAudio> {
                if value.is_nil() {
                    Err(InvalidAudio)
                } else {
                    Ok(Self(value))
                }
            }
            pub fn as_uuid(self) -> Uuid {
                self.0
            }
        }
    };
}
id!(TrackId);
id!(EntryId);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InvalidAudio;

/// A range already checked against the container's mdat by inspection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SampleRange {
    pub offset: u64,
    pub length: u64,
}

/// Structural evidence supplied by inspection, not proof of ownership or decoding.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidatedAudio {
    size_bytes: u64,
    duration_ms: u64,
    sample_rate: u32,
    channels: u16,
    decoder_config: Vec<u8>,
    samples: Vec<SampleRange>,
    digest: AudioDigest,
    store_id: VerifiedStoreId,
}

impl ValidatedAudio {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        size_bytes: u64,
        duration_ms: u64,
        sample_rate: u32,
        channels: u16,
        decoder_config: Vec<u8>,
        samples: Vec<SampleRange>,
        digest: AudioDigest,
        store_id: VerifiedStoreId,
    ) -> Result<Self, InvalidAudio> {
        if size_bytes == 0
            || duration_ms == 0
            || sample_rate == 0
            || channels == 0
            || decoder_config.is_empty()
            || samples.is_empty()
            || samples.iter().any(|s| {
                s.length == 0
                    || s.offset
                        .checked_add(s.length)
                        .is_none_or(|end| end > size_bytes)
            })
        {
            return Err(InvalidAudio);
        }
        Ok(Self {
            size_bytes,
            duration_ms,
            sample_rate,
            channels,
            decoder_config,
            samples,
            digest,
            store_id,
        })
    }
    pub fn size_bytes(&self) -> u64 {
        self.size_bytes
    }
    pub fn duration_ms(&self) -> u64 {
        self.duration_ms
    }
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
    pub fn channels(&self) -> u16 {
        self.channels
    }
    pub fn decoder_config(&self) -> &[u8] {
        &self.decoder_config
    }
    pub fn samples(&self) -> &[SampleRange] {
        &self.samples
    }
    pub fn digest(&self) -> &AudioDigest {
        &self.digest
    }
    pub fn store_id(&self) -> VerifiedStoreId {
        self.store_id
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrackState {
    Staging,
    Ready,
    Deleting,
    Deleted,
    Error,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrackEvent {
    Publish,
    Fail,
    BeginDelete,
    FinishDelete,
    RetryImport,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Track {
    id: TrackId,
    state: TrackState,
    audio: ValidatedAudio,
}
impl Track {
    pub fn new(id: TrackId, audio: ValidatedAudio) -> Self {
        Self {
            id,
            state: TrackState::Staging,
            audio,
        }
    }
    pub fn id(&self) -> TrackId {
        self.id
    }
    pub fn state(&self) -> TrackState {
        self.state
    }
    pub fn audio(&self) -> &ValidatedAudio {
        &self.audio
    }
    /// Caller must check persisted exclusions before RetryImport or Publish.
    pub fn transition(&mut self, event: TrackEvent) -> Result<(), InvalidAudio> {
        use TrackEvent::*;
        use TrackState::*;
        self.state = match (self.state, event) {
            (Staging, Publish) => Ready,
            (Staging | Ready, Fail) => Error,
            (Ready | Error | Staging, BeginDelete) => Deleting,
            (Deleting, FinishDelete) => Deleted,
            (Deleted | Error, RetryImport) => Staging,
            _ => return Err(InvalidAudio),
        };
        Ok(())
    }
}
