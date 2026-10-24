use rustune_contracts::{error::*, imports::*, library::*, settings::*, *};
use serde_json::{from_value, json, to_value};
use uuid::Uuid;
#[test]
fn patch_preserves_missing_null_and_value_in_both_directions() {
    let wire = json!({"expected_revision":12,"title":null,"artist":"  音楽  ","disc_number":2});
    let patch: MetadataPatch = from_value(wire.clone()).unwrap();
    assert_eq!(patch.title, Patch::Clear);
    assert_eq!(patch.album, Patch::Keep);
    assert_eq!(patch.artist, Patch::Set("  音楽  ".to_owned()));
    assert_eq!(to_value(&patch).unwrap(), wire);
    let valid = patch.validate().unwrap();
    assert_eq!(valid.artist, Patch::Set("音楽".to_owned()));
    assert!(to_value(Patch::<String>::Keep).is_err());
    assert!(from_value::<MetadataPatch>(json!({"title":"x"})).is_err());
    assert!(from_value::<MetadataPatch>(json!({"expected_revision":0,"buyer":"private"})).is_err());
}
#[test]
fn patch_validates_unicode_scalars_not_bytes_and_rejects_empty_and_invalid_order() {
    for title in ["", "  "] {
        assert!(
            from_value::<MetadataPatch>(json!({"expected_revision":0,"title":title}))
                .unwrap()
                .validate()
                .is_err()
        );
    }
    assert!(validate_text(&"音".repeat(512), 512).is_ok());
    assert!(validate_text(&"音".repeat(513), 512).is_err());
    assert!(validate_text(&"e\u{301}".repeat(256), 512).is_ok());
    for n in [0, 1000] {
        assert!(
            from_value::<MetadataPatch>(json!({"expected_revision":0,"track_number":n}))
                .unwrap()
                .validate()
                .is_err()
        );
    }
    assert!(from_value::<MetadataPatch>(json!({"expected_revision":-1})).is_err());
    assert!(
        Pagination {
            offset: 0,
            limit: 101
        }
        .validate()
        .is_err()
    );
    assert!(
        Pagination {
            offset: u64::MAX,
            limit: 1
        }
        .validate()
        .is_err()
    );
    assert!(
        CreatePlaylist {
            name: "音".repeat(121)
        }
        .validate()
        .is_err()
    );
}
#[test]
fn pagination_applies_defaults_for_omitted_fields_and_caps_offset() {
    let default = Pagination {
        offset: 0,
        limit: 50,
    };
    assert_eq!(from_value::<Pagination>(json!({})).unwrap(), default);
    assert_eq!(
        from_value::<Pagination>(json!({"offset":20})).unwrap(),
        Pagination {
            offset: 20,
            limit: 50
        }
    );
    assert_eq!(
        from_value::<Pagination>(json!({"limit":10})).unwrap(),
        Pagination {
            offset: 0,
            limit: 10
        }
    );
    // 明示値は既定値で上書きされず、そのまま検証される。
    assert!(
        from_value::<Pagination>(json!({"limit":0}))
            .unwrap()
            .validate()
            .is_err()
    );
    assert!(from_value::<Pagination>(json!({"page":1})).is_err());
    let max = Pagination {
        offset: Pagination::MAX_OFFSET,
        limit: 50,
    };
    assert_eq!(max.validate(), Ok(max));
    assert!(
        Pagination {
            offset: Pagination::MAX_OFFSET + 1,
            limit: 50
        }
        .validate()
        .is_err()
    );
    assert!(
        Pagination {
            offset: 0,
            limit: Pagination::MAX_LIMIT
        }
        .validate()
        .is_ok()
    );
}
#[test]
fn dto_roundtrip_does_not_require_domain_or_raw_tags() {
    let summary = TrackSummary {
        id: Uuid::from_u128(1),
        title: "曲".into(),
        artist: "歌手".into(),
        album: "アルバム".into(),
        duration_ms: 1000,
        artwork_id: None,
        favorite: true,
        revision: 4,
    };
    let page = Page {
        items: vec![summary],
        total: 1,
        offset: 0,
        limit: 50,
        library_revision: 8,
    };
    let wire = to_value(&page).unwrap();
    assert_eq!(
        from_value::<Page<TrackSummary>>(wire.clone()).unwrap(),
        page
    );
    let track = wire["items"][0].as_object().unwrap();
    assert_eq!(track.len(), 8);
    assert!(!track.contains_key("path"));
    assert!(!track.contains_key("account"));
    let item = ImportItem {
        id: Uuid::from_u128(2),
        source_root_name: "Music".into(),
        relative_path: "album/song.m4a".into(),
        status: ItemStatus::Skipped,
        reason: Some("future_reason".into()),
        retryable: false,
        track_id: None,
    };
    assert_eq!(
        from_value::<ImportItem>(to_value(&item).unwrap()).unwrap(),
        item
    );
}
#[test]
fn unknown_error_codes_survive_for_ui_fallback_and_messages_are_fixed() {
    let future = json!({"code":"future_code","message":"処理できません。","retryable":false,"request_id":"request-1"});
    let parsed: ApiError = from_value(future.clone()).unwrap();
    assert_eq!(to_value(parsed).unwrap(), future);
    let known = ErrorCode::IdentityConflict.response(false, "request-2".into());
    assert_eq!(known.code, "identity_conflict");
    assert_eq!(known.message, "楽曲の識別情報が競合しています。");
}
#[test]
fn playlist_entries_can_repeat_tracks_but_not_entry_ids() {
    let a = PlaylistItem {
        id: Uuid::from_u128(1),
        track_id: Uuid::from_u128(9),
    };
    let b = PlaylistItem {
        id: Uuid::from_u128(2),
        track_id: a.track_id,
    };
    let request = ReplacePlaylistItems {
        expected_revision: 1,
        items: vec![a.clone(), b],
    };
    assert!(request.validate().is_ok());
    assert!(
        ReplacePlaylistItems {
            expected_revision: 1,
            items: vec![a.clone(), a]
        }
        .validate()
        .is_err()
    );
}
#[test]
fn settings_have_bounded_nonnullable_values() {
    let valid:SettingsPatch=from_value(json!({"expected_revision":0,"interval_seconds":60,"max_file_bytes":209715200,"reserve_free_bytes":1073741824,"storefront":"jp"})).unwrap();
    assert!(valid.validate().is_ok());
    for value in [
        json!({"auto_import":null}),
        json!({"interval_seconds":29}),
        json!({"max_file_bytes":1}),
        json!({"reserve_free_bytes":0}),
        json!({"storefront":"JPN"}),
        json!({"source_root":" "}),
    ] {
        let mut value = value;
        value["expected_revision"] = json!(0);
        assert!(
            from_value::<SettingsPatch>(value)
                .unwrap()
                .validate()
                .is_err()
        );
    }
}
