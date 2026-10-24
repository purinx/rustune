use super::*;
use mp4_track::{AacConfig, Mp4Writer, SampleInput, TrackConfig, WriterConfig};
use std::io::Cursor;

fn fixture(packet: &[u8], purchased: bool) -> Vec<u8> {
    let mut writer = Mp4Writer::new(Cursor::new(Vec::new()), WriterConfig::default()).unwrap();
    let track = writer
        .add_track(TrackConfig {
            kind: TrackKind::Audio,
            timescale: 44100,
            language: "und".into(),
            handler_name: "synthetic".into(),
            width: 0.0,
            height: 0.0,
            edit_list: vec![],
            codec: Codec::Aac {
                config: AacConfig {
                    raw_esds: vec![
                        0, 0, 0, 0, 3, 25, 0, 1, 0, 4, 17, 0x40, 0x15, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                        0, 0, 5, 2, 0x12, 0x10, 6, 1, 2,
                    ],
                    object_type: 0x40,
                    audio_specific_config: vec![0x12, 0x10],
                    channels: 2,
                    sample_rate: 44100,
                    sidecars: vec![],
                },
            },
        })
        .unwrap();
    writer
        .write_sample(
            track,
            &SampleInput {
                data: packet,
                duration: 1024,
                cts_offset: 0,
                is_sync: true,
            },
        )
        .unwrap();
    let mut memory = writer.finish().unwrap();
    let mut tag = mp4ameta::Tag::read_from(&mut memory).unwrap();
    tag.set_title("Same title");
    tag.set_artist("Synthetic artist");
    if purchased {
        tag.set_data(
            mp4ameta::Fourcc(*b"purd"),
            mp4ameta::Data::Utf8("2026-10-18".into()),
        );
        tag.set_data(
            mp4ameta::Fourcc(*b"apID"),
            mp4ameta::Data::Utf8("private@example.invalid".into()),
        );
        tag.set_data(
            mp4ameta::Fourcc(*b"cnID"),
            mp4ameta::Data::BeSigned(123_u32.to_be_bytes().to_vec()),
        );
    }
    tag.write_to(&mut memory).unwrap();
    memory.into_inner()
}
fn accepted(bytes: &[u8]) -> InspectedAudio {
    match inspect(&mut Cursor::new(bytes), InspectionLimits::default()) {
        InspectionResult::Accepted(audio) => *audio,
        result => panic!("not accepted: {result:?}"),
    }
}
fn rejected(bytes: &[u8], limits: InspectionLimits) -> Reason {
    match inspect(&mut Cursor::new(bytes), limits) {
        InspectionResult::Rejected(reason) => reason,
        result => panic!("not rejected: {result:?}"),
    }
}
#[test]
fn validates_purchase_aac_and_strips_private_tags_from_result() {
    let audio = accepted(&fixture(&[1, 2, 3], true));
    assert_eq!(audio.audio.store_id().value(), 123);
    assert_eq!(audio.audio.samples().len(), 1);
    assert_eq!(audio.audio.sample_rate(), 44100);
    assert_eq!(audio.metadata.title.unwrap().as_str(), "Same title");
    assert_eq!(audio.parser_version, PARSER_VERSION);
    assert!(!format!("{:?}", audio.audio).contains("private@example.invalid"));
}
#[test]
fn rejects_cd_like_aac_and_invalid_purchase_tags() {
    assert_eq!(
        rejected(&fixture(&[1, 2, 3], false), InspectionLimits::default()),
        Reason::PurchaseUnverified
    );
    let mut memory = Cursor::new(fixture(&[1, 2, 3], true));
    let mut tag = mp4ameta::Tag::read_from(&mut memory).unwrap();
    tag.set_data(
        mp4ameta::Fourcc(*b"cnID"),
        mp4ameta::Data::BeSigned(vec![0; 4]),
    );
    tag.write_to(&mut memory).unwrap();
    assert_eq!(
        rejected(memory.get_ref(), InspectionLimits::default()),
        Reason::PurchaseUnverified
    );
}
#[test]
fn identity_ignores_tags_but_distinguishes_audio() {
    let bytes = fixture(&[1, 2, 3], true);
    let a = accepted(&bytes);
    let mut memory = Cursor::new(bytes);
    let mut tag = mp4ameta::Tag::read_from(&mut memory).unwrap();
    tag.set_title("A different longer title");
    tag.write_to(&mut memory).unwrap();
    let b = accepted(memory.get_ref());
    let c = accepted(&fixture(&[1, 2, 4], true));
    assert_eq!(a.audio.digest(), b.audio.digest());
    assert_ne!(a.audio.digest(), c.audio.digest());
    assert_eq!(a.audio.store_id(), c.audio.store_id());
}
#[test]
fn rejects_truncation_and_outside_mdat_sample_offsets() {
    let bytes = fixture(&[1, 2, 3], true);
    assert_eq!(
        rejected(&bytes[..bytes.len() - 1], InspectionLimits::default()),
        Reason::InvalidContainer
    );
    for offset in [8, u32::MAX] {
        let mut invalid = bytes.clone();
        let at = invalid.windows(4).position(|v| v == b"stco").unwrap();
        invalid[at + 12..at + 16].copy_from_slice(&offset.to_be_bytes());
        assert_eq!(
            rejected(&invalid, InspectionLimits::default()),
            Reason::InvalidContainer
        );
    }
}
#[test]
fn enforces_file_metadata_sample_box_and_depth_limits() {
    let bytes = fixture(&[1, 2, 3], true);
    let default = InspectionLimits::default();
    for limits in [
        InspectionLimits {
            max_file_bytes: 1,
            ..default
        },
        InspectionLimits {
            max_metadata_bytes: 16,
            ..default
        },
        InspectionLimits {
            max_sample_bytes: 2,
            ..default
        },
        InspectionLimits {
            max_samples: 0,
            ..default
        },
        InspectionLimits {
            max_boxes: 3,
            ..default
        },
        InspectionLimits {
            max_depth: 2,
            ..default
        },
    ] {
        assert_eq!(rejected(&bytes, limits), Reason::AnalysisLimit);
    }
}
#[test]
fn nested_boxes_use_one_shared_budget() {
    let mut bytes = fixture(&[1, 2, 3], true);
    // top-levelに入れ子のudtaを追加し、全階層で共有する予算と深さ上限を確かめる。
    let mut nested = Vec::new();
    for _ in 0..35 {
        let mut outer = ((nested.len() + 8) as u32).to_be_bytes().to_vec();
        outer.extend_from_slice(b"udta");
        outer.extend(nested);
        nested = outer;
    }
    bytes.extend(nested);
    assert_eq!(
        rejected(&bytes, InspectionLimits::default()),
        Reason::AnalysisLimit
    );
}
#[test]
fn bounded_reader_disallows_out_of_range_seeks() {
    let mut input = Cursor::new(vec![1, 2, 3]);
    let mut bounded = bounds::Bounded::new(&mut input, 3).unwrap();
    assert!(bounded.seek(SeekFrom::Start(4)).is_err());
    assert!(bounded.seek(SeekFrom::End(-4)).is_err());
    bounded.seek(SeekFrom::Start(2)).unwrap();
    let mut bytes = [0; 16];
    assert_eq!(bounded.read(&mut bytes).unwrap(), 1);
    assert_eq!(bounded.read(&mut bytes).unwrap(), 0);
}
#[test]
fn inspection_source_never_modifies_original_and_detects_mutation() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.m4a");
    let bytes = fixture(&[1, 2, 3], true);
    std::fs::write(&path, &bytes).unwrap();
    let before = std::fs::metadata(&path).unwrap();
    let root = crate::source::SourceRoot::open(dir.path()).unwrap();
    let mut source = root
        .open_readonly(std::path::Path::new("test.m4a"))
        .unwrap();
    assert!(matches!(
        inspect_source(&mut source, InspectionLimits::default()),
        InspectionResult::Accepted(_)
    ));
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    let after = std::fs::metadata(&path).unwrap();
    assert_eq!(before.len(), after.len());
    assert_eq!(before.modified().unwrap(), after.modified().unwrap());
    std::fs::write(&path, b"changed").unwrap();
    assert!(matches!(
        inspect_source(&mut source, InspectionLimits::default()),
        InspectionResult::Deferred(Reason::SourceChanged)
    ));
}

/// 端末内で明示指定された実音源だけを使う。CIでは実行しない。
/// `RUSTUNE_AUDIO_TEST_FILE`に指定ファイルの絶対パスを設定し`--ignored`で実行する。
/// 購入者タグの値・パス・曲名は出力しない。
#[test]
#[ignore = "端末内の指定実音源専用。CIでは実行しない"]
fn authorized_local_audio_is_accepted() {
    let path = std::path::PathBuf::from(
        std::env::var_os("RUSTUNE_AUDIO_TEST_FILE").expect("指定ファイルのパスが必要"),
    );
    let root = crate::source::SourceRoot::open(path.parent().unwrap()).expect("root");
    let name = std::path::Path::new(path.file_name().unwrap());
    let mut source = root.open_readonly(name).expect("open");
    let before = source.initial_fingerprint();
    let result = inspect_source(&mut source, InspectionLimits::default());
    let InspectionResult::Accepted(audio) = result else {
        panic!("not accepted: {result:?}");
    };
    assert!(audio.audio.samples().len() > 1000);
    assert!(audio.audio.duration_ms() > 0);
    assert_ne!(audio.audio.store_id().value(), 0);
    assert_eq!(audio.parser_version, PARSER_VERSION);
    root.verify_entry(name, &source).expect("unchanged");
    assert_eq!(source.initial_fingerprint(), before);
}

fn box_at(bytes: &[u8], kind: &[u8; 4]) -> usize {
    bytes.windows(4).position(|v| v == kind).unwrap() - 4
}
fn set_size(bytes: &mut [u8], at: usize, size: u32) {
    bytes[at..at + 4].copy_from_slice(&size.to_be_bytes());
}
fn size_of(bytes: &[u8], at: usize) -> u32 {
    u32::from_be_bytes(bytes[at..at + 4].try_into().unwrap())
}
fn retag(bytes: Vec<u8>, edit: impl FnOnce(&mut mp4ameta::Tag)) -> Vec<u8> {
    let mut memory = Cursor::new(bytes);
    let mut tag = mp4ameta::Tag::read_from(&mut memory).unwrap();
    edit(&mut tag);
    tag.write_to(&mut memory).unwrap();
    memory.into_inner()
}

#[test]
fn rejects_empty_trailing_and_broken_box_lengths() {
    let default = InspectionLimits::default();
    assert_eq!(rejected(&[], default), Reason::InvalidContainer);
    let bytes = fixture(&[1, 2, 3], true);
    // 8 bytes未満の余りはbox headerにならない。
    let mut trailing = bytes.clone();
    trailing.extend_from_slice(&[0, 0, 0]);
    assert_eq!(rejected(&trailing, default), Reason::InvalidContainer);
    let moov = box_at(&bytes, b"moov");
    for size in [4, u32::MAX, size_of(&bytes, moov) + 1] {
        let mut broken = bytes.clone();
        set_size(&mut broken, moov, size);
        assert_eq!(
            rejected(&broken, default),
            Reason::InvalidContainer,
            "{size}"
        );
    }
    // 子boxが親の範囲を超える。
    let mut child = bytes.clone();
    let stco = box_at(&bytes, b"stco");
    set_size(&mut child, stco, size_of(&bytes, stco) + 64);
    assert_eq!(rejected(&child, default), Reason::InvalidContainer);
    // 64-bit largesizeの加算overflow。
    let mut large = bytes.clone();
    let free = box_at(&bytes, b"free");
    set_size(&mut large, free, 1);
    let mut patched = large[..free + 8].to_vec();
    patched.extend_from_slice(&u64::MAX.to_be_bytes());
    patched.extend_from_slice(&large[free + 8..]);
    assert_eq!(rejected(&patched, default), Reason::InvalidContainer);
}

#[test]
fn rejects_sample_size_beyond_mdat() {
    let bytes = fixture(&[1, 2, 3], true);
    let stsz = box_at(&bytes, b"stsz");
    // stsz: header 8 + version/flags 4 + sample_size 4 + count 4 + entry 4。
    for size in [4_u32, 1024 * 1024 + 1] {
        let mut invalid = bytes.clone();
        invalid[stsz + 20..stsz + 24].copy_from_slice(&size.to_be_bytes());
        let reason = rejected(&invalid, InspectionLimits::default());
        assert!(
            matches!(reason, Reason::InvalidContainer | Reason::AnalysisLimit),
            "{reason:?}"
        );
    }
}

#[test]
fn rejects_non_aac_entries_and_fragmented_movies() {
    let bytes = fixture(&[1, 2, 3], true);
    let mp4a = box_at(&bytes, b"mp4a");
    // ALAC相当のsample entryはAACとして扱わない。
    let mut alac = bytes.clone();
    alac[mp4a + 4..mp4a + 8].copy_from_slice(b"alac");
    assert_eq!(
        rejected(&alac, InspectionLimits::default()),
        Reason::UnsupportedCodec
    );
    // version 1以上のsound descriptionは未対応。
    let mut version = bytes.clone();
    version[mp4a + 16..mp4a + 18].copy_from_slice(&[0, 1]);
    assert_eq!(
        rejected(&version, InspectionLimits::default()),
        Reason::UnsupportedCodec
    );
    let mut fragmented = bytes.clone();
    let udta = box_at(&bytes, b"udta");
    fragmented[udta + 4..udta + 8].copy_from_slice(b"mvex");
    assert_eq!(
        rejected(&fragmented, InspectionLimits::default()),
        Reason::UnsupportedCodec
    );
}

#[test]
fn purchase_rule_rejects_missing_blank_oversized_duplicate_and_bad_store_ids() {
    let base = fixture(&[1, 2, 3], true);
    type Edit = Box<dyn FnOnce(&mut mp4ameta::Tag)>;
    let cases: Vec<Edit> = vec![
        Box::new(|t| t.remove_data_of(&mp4ameta::Fourcc(*b"purd"))),
        Box::new(|t| t.remove_data_of(&mp4ameta::Fourcc(*b"apID"))),
        Box::new(|t| t.remove_data_of(&mp4ameta::Fourcc(*b"cnID"))),
        Box::new(|t| {
            t.set_data(
                mp4ameta::Fourcc(*b"apID"),
                mp4ameta::Data::Utf8("   ".into()),
            )
        }),
        Box::new(|t| {
            t.set_data(
                mp4ameta::Fourcc(*b"purd"),
                mp4ameta::Data::Utf8("x".repeat(1025)),
            )
        }),
        Box::new(|t| {
            t.add_data(
                mp4ameta::Fourcc(*b"apID"),
                mp4ameta::Data::Utf8("second@example.invalid".into()),
            )
        }),
        Box::new(|t| {
            t.set_data(
                mp4ameta::Fourcc(*b"purd"),
                mp4ameta::Data::BeSigned(vec![1, 2, 3, 4]),
            )
        }),
        Box::new(|t| {
            t.set_data(
                mp4ameta::Fourcc(*b"cnID"),
                mp4ameta::Data::BeSigned(vec![0, 0, 0, 0, 1]),
            )
        }),
    ];
    for (index, edit) in cases.into_iter().enumerate() {
        let bytes = retag(base.clone(), edit);
        assert_eq!(
            rejected(&bytes, InspectionLimits::default()),
            Reason::PurchaseUnverified,
            "case {index}"
        );
    }
}

#[test]
fn quicktime_style_meta_is_structurally_valid_but_not_purchase_evidence() {
    let mut bytes = fixture(&[1, 2, 3], true);
    // mdatがmoovより前にあるため、meta内の4 bytesを除いてもsample offsetは変わらない。
    let (moov, udta, meta) = (
        box_at(&bytes, b"moov"),
        box_at(&bytes, b"udta"),
        box_at(&bytes, b"meta"),
    );
    assert!(box_at(&bytes, b"mdat") < moov);
    for at in [moov, udta, meta] {
        let size = size_of(&bytes, at);
        set_size(&mut bytes, at, size - 4);
    }
    bytes.drain(meta + 8..meta + 12);
    let size = bytes.len() as u64;
    assert!(bounds::validate(&mut Cursor::new(&bytes), size, InspectionLimits::default()).is_ok());
    // mp4ameta 0.13.0はQuickTime形式metaを読めないため、購入由来を確認できない扱い。
    assert_eq!(
        rejected(&bytes, InspectionLimits::default()),
        Reason::PurchaseUnverified
    );
}

#[test]
fn multiple_artwork_data_does_not_break_track_parsing() {
    let bytes = retag(fixture(&[1, 2, 3], true), |t| {
        t.add_artwork(mp4ameta::Img::png(vec![0x89, b'P', b'N', b'G']));
        t.add_artwork(mp4ameta::Img::jpeg(vec![0xff, 0xd8, 0xff]));
    });
    let audio = accepted(&bytes);
    assert_eq!(audio.artwork.len(), 1);
    assert_eq!(audio.artwork[0].bytes, vec![0x89, b'P', b'N', b'G']);
}

#[test]
fn digest_follows_task02_probe_scheme() {
    let audio = accepted(&fixture(&[1, 2, 3], true));
    let mut expected = Sha256::new();
    expected.update(b"rustune-aac-probe-v1\0");
    for part in [&[0x12_u8, 0x10][..], &[1, 2, 3][..]] {
        expected.update((part.len() as u64).to_be_bytes());
        expected.update(part);
    }
    let expected: [u8; 32] = expected.finalize().into();
    assert_eq!(audio.audio.digest().bytes(), &expected);
    assert_eq!(audio.audio.digest().scheme(), "rustune-aac-probe-v1");
}

#[test]
fn reasons_map_to_contract_codes_without_private_values() {
    use rustune_contracts::error::ErrorCode;
    let expected = [
        (Reason::ReadFailed, ErrorCode::IoFailure, true),
        (Reason::InvalidContainer, ErrorCode::CorruptAudio, true),
        (Reason::UnsupportedCodec, ErrorCode::UnsupportedCodec, false),
        (
            Reason::PurchaseUnverified,
            ErrorCode::PurchaseUnverified,
            false,
        ),
        (Reason::AnalysisLimit, ErrorCode::UnsupportedCodec, false),
        (Reason::SourceChanged, ErrorCode::FileUnstable, true),
    ];
    for (reason, code, retryable) in expected {
        assert_eq!(reason.error_code(), code);
        assert_eq!(reason.retryable(), retryable);
    }
    let audio = accepted(&fixture(&[1, 2, 3], true));
    let debug = format!("{audio:?}");
    assert!(!debug.contains("private@example.invalid"));
    assert!(!debug.contains("2026-10-18"));
}
