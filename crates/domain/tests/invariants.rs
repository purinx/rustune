use rustune_domain::{identity::*, import_state::*, metadata::*, queue::*, track::*};
use uuid::Uuid;

fn track(n: u128) -> TrackId {
    TrackId::new(Uuid::from_u128(n)).unwrap()
}
fn entry(n: u128, t: u128) -> QueueEntry {
    QueueEntry {
        id: EntryId::new(Uuid::from_u128(n)).unwrap(),
        track_id: track(t),
    }
}
fn audio(samples: Vec<SampleRange>) -> Result<ValidatedAudio, InvalidAudio> {
    ValidatedAudio::new(
        100,
        1000,
        44100,
        2,
        vec![0x12, 0x10],
        samples,
        AudioDigest::new([1; 32]),
        VerifiedStoreId::from_matched_tag_pattern(12).unwrap(),
    )
}

#[test]
fn invalid_audio_never_becomes_publishable() {
    for ranges in [
        vec![],
        vec![SampleRange {
            offset: 90,
            length: 11,
        }],
        vec![SampleRange {
            offset: u64::MAX,
            length: 2,
        }],
        vec![SampleRange {
            offset: 1,
            length: 0,
        }],
    ] {
        assert!(audio(ranges).is_err());
    }
    let mut t = Track::new(
        track(1),
        audio(vec![SampleRange {
            offset: 20,
            length: 80,
        }])
        .unwrap(),
    );
    assert!(t.transition(TrackEvent::FinishDelete).is_err());
    t.transition(TrackEvent::Publish).unwrap();
    assert_eq!(t.state(), TrackState::Ready);
    assert!(t.transition(TrackEvent::Publish).is_err());
    t.transition(TrackEvent::BeginDelete).unwrap();
    assert!(t.transition(TrackEvent::Publish).is_err());
    t.transition(TrackEvent::FinishDelete).unwrap();
    assert!(t.transition(TrackEvent::Publish).is_err());
    t.transition(TrackEvent::RetryImport).unwrap();
    assert_eq!(t.state(), TrackState::Staging);
    assert!(TrackId::new(Uuid::nil()).is_err());
    assert!(VerifiedStoreId::from_matched_tag_pattern(0).is_none());
}

#[test]
fn identity_never_merges_conflicting_audio_or_tracks() {
    let d = AudioDigest::new([1; 32]);
    let m = IdentityMatch {
        track_id: track(1),
        state: TrackState::Ready,
        digest: d.clone(),
    };
    assert_eq!(reconcile_identity(&d, &[]), IdentityDecision::New);
    assert_eq!(
        reconcile_identity(&d, std::slice::from_ref(&m)),
        IdentityDecision::Existing(track(1))
    );
    assert_eq!(
        reconcile_identity(&AudioDigest::new([2; 32]), std::slice::from_ref(&m)),
        IdentityDecision::Conflict
    );
    assert_eq!(
        reconcile_identity(
            &d,
            &[
                m.clone(),
                IdentityMatch {
                    track_id: track(2),
                    ..m.clone()
                }
            ]
        ),
        IdentityDecision::Conflict
    );
    assert_eq!(
        reconcile_identity(
            &d,
            &[IdentityMatch {
                state: TrackState::Deleted,
                ..m
            }]
        ),
        IdentityDecision::Excluded
    );
    let keys = resolve_identity(VerifiedStoreId::from_matched_tag_pattern(12), d);
    assert!(matches!(
        keys.as_slice(),
        [IdentityKey::Store(_), IdentityKey::Audio(_)]
    ));
}

#[test]
fn display_priority_survives_rescan_and_clear_returns_to_import() {
    let fallback = Text::new("filename").unwrap();
    let mut imported = Metadata {
        title: Some(Text::new("embedded").unwrap()),
        ..Default::default()
    };
    let enriched = Metadata {
        title: Some(Text::new("enriched").unwrap()),
        ..Default::default()
    };
    let mut manual = Metadata {
        title: Some(Text::new("manual").unwrap()),
        ..Default::default()
    };
    imported.title = Some(Text::new("rescan").unwrap());
    let effective = resolve_metadata(&imported, &enriched, &manual, &fallback);
    assert_eq!(effective.title.value.as_str(), "manual");
    assert_eq!(effective.title.origin, MetadataOrigin::Manual);
    manual.title = None;
    assert_eq!(
        resolve_metadata(&imported, &enriched, &manual, &fallback)
            .title
            .origin,
        MetadataOrigin::Enriched
    );
    assert_eq!(
        resolve_metadata(&imported, &Metadata::default(), &manual, &fallback)
            .title
            .value
            .as_str(),
        "rescan"
    );
    assert_eq!(
        resolve_metadata(
            &Metadata::default(),
            &Metadata::default(),
            &manual,
            &fallback
        )
        .title
        .origin,
        MetadataOrigin::Fallback
    );
    assert_eq!(
        normalize_search("ＡＢＣ ｶﾞ カ\u{3099} Café"),
        "abc ガ ガ café"
    );
    assert_ne!(normalize_search("ひらがな"), normalize_search("ヒラガナ"));
    assert!(Text::new(" \t ").is_err());
    assert!(Text::new(&"音".repeat(512)).is_ok());
    assert!(Text::new(&"音".repeat(513)).is_err());
    assert!(TrackNumber::new(0).is_err());
    assert!(TrackNumber::new(1000).is_err());
}

#[test]
fn terminal_import_states_cannot_finish_twice_or_retry_in_place() {
    let running = transition_job(JobState::Queued, JobEvent::Start).unwrap();
    let done = transition_job(running, JobEvent::Complete).unwrap();
    for event in [
        JobEvent::Start,
        JobEvent::Complete,
        JobEvent::Fail,
        JobEvent::Interrupt,
    ] {
        assert!(transition_job(done, event).is_err());
    }
    assert_eq!(
        transition_job(running, JobEvent::Interrupt),
        Ok(JobState::Failed)
    );
    for state in [ItemState::Imported, ItemState::Skipped, ItemState::Failed] {
        for event in [
            ItemEvent::Check,
            ItemEvent::Copy,
            ItemEvent::Publish,
            ItemEvent::Skip,
            ItemEvent::Fail,
            ItemEvent::Interrupt,
        ] {
            assert!(transition_item(state, event).is_err());
        }
    }
    assert!(transition_item(ItemState::Pending, ItemEvent::Publish).is_err());
    assert_eq!(
        transition_item(ItemState::Copying, ItemEvent::Interrupt),
        Ok(ItemState::Failed)
    );
    assert!(Progress::new(Some(2), 1, 1, 1).is_err());
    assert!(Progress::new(None, u64::MAX, 1, 0).is_err());
    assert_eq!(Progress::new(Some(3), 1, 1, 1).unwrap().processed(), 3);
}

#[test]
fn manual_album_edit_releases_store_group_and_compilation_has_stable_fallback() {
    let fallback = Text::new("filename").unwrap();
    let imported = Metadata {
        album: Some(Text::new("Album").unwrap()),
        artist: Some(Text::new("Singer").unwrap()),
        ..Default::default()
    };
    let store = std::num::NonZeroU64::new(123);
    let effective = resolve_metadata(
        &imported,
        &Metadata::default(),
        &Metadata::default(),
        &fallback,
    );
    assert_eq!(
        album_key(&effective, store, false, false),
        AlbumKey::Store(store.unwrap())
    );
    assert_eq!(
        album_key(&effective, store, true, true),
        AlbumKey::Metadata {
            album: "album".into(),
            artist: "various artists".into()
        }
    );
    let manual = Metadata {
        album: Some(Text::new("Edited").unwrap()),
        ..Default::default()
    };
    let effective = resolve_metadata(&imported, &Metadata::default(), &manual, &fallback);
    assert_eq!(
        album_key(&effective, store, false, false),
        AlbumKey::Metadata {
            album: "edited".into(),
            artist: "singer".into()
        }
    );
}

#[test]
fn duplicate_track_entries_keep_distinct_selection_and_end_stops() {
    let mut q = Queue::default();
    assert_eq!(q.next_entry(), QueueEffect::Stop);
    assert_eq!(q.previous(), QueueEffect::Stop);
    let a = entry(1, 11);
    let b = entry(2, 11);
    let c = entry(3, 12);
    q.replace_and_select(vec![a, b, c], Some(a.id)).unwrap();
    assert_eq!(q.previous(), QueueEffect::Play(a));
    assert_eq!(q.next_entry(), QueueEffect::Play(b));
    assert_eq!(q.remove_track(track(11)), QueueEffect::Play(c));
    assert_eq!(q.entries(), &[c]);
    assert_eq!(q.next_entry(), QueueEffect::Stop);
    assert_eq!(q.current(), None);
    assert_eq!(q.remove_track(track(12)), QueueEffect::None);
    assert!(q.entries().is_empty());
}

#[test]
fn queue_rejects_invalid_selection_without_mutation_and_removal_of_last_stops() {
    let mut q = Queue::default();
    let a = entry(1, 11);
    let b = entry(2, 12);
    q.replace_and_select(vec![a, b], Some(b.id)).unwrap();
    let old = q.clone();
    assert!(q.replace_and_select(vec![a, a], Some(a.id)).is_err());
    assert_eq!(q, old);
    assert!(q.replace_and_select(vec![a], Some(b.id)).is_err());
    assert_eq!(q, old);
    assert!(q.append(b).is_err());
    assert_eq!(q.remove_track(b.track_id), QueueEffect::Stop);
    assert_eq!(q.current(), None);
    assert_eq!(q.entries(), &[a]);
}
