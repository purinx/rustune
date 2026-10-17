//! Local AAC feasibility diagnostics, not cryptographic proof of ownership.
use mp4_atom::ReadFrom;
use mp4_track::{Codec, Limits, Mp4Reader, TrackKind};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::io::{Cursor, Read};
use std::ops::Range;

const MAX_FILE_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Serialize)]
struct Probe {
    parser: &'static str,
    purchase_rule: &'static str,
    purchase_evidence: &'static str,
    primary_entry: &'static str,
    samples: u32,
    warning_count: usize,
    purchase_date_tag_present: bool,
    account_tag_present: bool,
    store_song_id: Option<u32>,
    audio_digest_scheme: Option<&'static str>,
    audio_digest: Option<String>,
}

// Use the existing parser's header decoder; this adapter only bounds top-level
// boxes and records mdat payload ranges. Nested sample tables remain library-owned.
fn media_ranges(bytes: &[u8]) -> Result<Vec<Range<u64>>, &'static str> {
    let mut input = Cursor::new(bytes);
    let mut ranges = Vec::new();
    let mut count = 0;
    while input.position() < bytes.len() as u64 {
        count += 1;
        if count > 100_000 {
            return Err("top_level_box_limit");
        }
        let header = mp4_atom::Header::read_from(&mut input).map_err(|_| "box_header_invalid")?;
        let start = input.position();
        let end = match header.size {
            Some(size) => start.checked_add(size as u64).ok_or("box_range_invalid")?,
            None => bytes.len() as u64,
        };
        if end > bytes.len() as u64 {
            return Err("box_range_invalid");
        }
        if header.kind == mp4_atom::FourCC::new(b"moov") && end - start > limits().max_moov_bytes {
            return Err("movie_size_limit");
        }
        if header.kind == mp4_atom::FourCC::new(b"mdat") {
            ranges.push(start..end);
        }
        input.set_position(end);
    }
    if ranges.is_empty() {
        return Err("media_data_missing");
    }
    Ok(ranges)
}

fn single_nonempty_text(tag: &mp4ameta::Tag, code: [u8; 4]) -> bool {
    let ident = mp4ameta::Fourcc(code);
    let mut values = tag.data_of(&ident);
    let valid = values
        .next()
        .and_then(mp4ameta::Data::string)
        .is_some_and(|s| !s.trim().is_empty() && s.len() <= 1024);
    valid && values.next().is_none()
}

fn song_id(tag: &mp4ameta::Tag) -> Option<u32> {
    let ident = mp4ameta::Fourcc(*b"cnID");
    let mut values = tag.data_of(&ident);
    let value = values.next()?.bytes()?;
    let id = u32::from_be_bytes(value.try_into().ok()?);
    (id != 0 && values.next().is_none()).then_some(id)
}

fn limits() -> Limits {
    let mut limits = Limits::default();
    limits.max_moov_bytes = 16 * 1024 * 1024;
    limits.max_moof_bytes = 1024 * 1024;
    limits.max_sample_bytes = 1024 * 1024;
    limits.max_fragments = 0;
    limits.max_sample_count = 1_000_000;
    limits.max_depth = 32;
    limits
}

fn bounded_read(reader: impl Read) -> Result<Vec<u8>, &'static str> {
    let mut bytes = Vec::new();
    reader
        .take(MAX_FILE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "read_failed")?;
    if bytes.len() as u64 > MAX_FILE_BYTES {
        return Err("file_limit");
    }
    Ok(bytes)
}

fn hash_part(hash: &mut Sha256, bytes: &[u8]) {
    hash.update((bytes.len() as u64).to_be_bytes());
    hash.update(bytes);
}

fn probe(bytes: &[u8]) -> Result<Probe, &'static str> {
    if bytes.len() as u64 > MAX_FILE_BYTES {
        return Err("file_limit");
    }
    let ranges = media_ranges(bytes)?;
    let mut reader = Mp4Reader::open_with(Cursor::new(bytes), limits())
        .map_err(|_| "container_invalid_or_unsupported")?;
    if reader.is_fragmented() {
        return Err("fragmented_audio_unsupported");
    }
    let tracks: Vec<_> = reader
        .tracks()
        .iter()
        .filter(|t| t.kind == TrackKind::Audio)
        .collect();
    if tracks.len() != 1 {
        return Err("single_audio_track_required");
    }
    let track = tracks[0];
    let id = track.id;
    let samples = track.sample_count;
    if samples == 0 {
        return Err("empty_audio");
    }
    let (entry, config) = match &track.codec {
        Codec::Aac { config }
            if config.object_type == 0x40 && !config.audio_specific_config.is_empty() =>
        {
            ("aac", Some(config.audio_specific_config.clone()))
        }
        _ => ("unsupported", None),
    };
    // The candidate exposes only the first codec description. Never digest
    // samples using another description as if its configuration were known.
    for sample in reader.samples(id).map_err(|_| "sample_table_invalid")? {
        if sample.description_index != 1 {
            return Err("multiple_descriptions_unsupported");
        }
        if sample.size == 0
            || sample.size > limits().max_sample_bytes
            || sample
                .offset
                .checked_add(u64::from(sample.size))
                .is_none_or(|end| end > bytes.len() as u64)
        {
            return Err("sample_range_invalid");
        }
        let end = sample.offset + u64::from(sample.size);
        if !ranges
            .iter()
            .any(|range| sample.offset >= range.start && end <= range.end)
        {
            return Err("sample_outside_media_data");
        }
    }
    let warning_count = reader.warnings().len();
    let digest = if let Some(config) = config {
        let mut hash = Sha256::new();
        hash.update(b"rustune-aac-probe-v1\0");
        hash_part(&mut hash, &config);
        for index in 0..samples {
            let sample = reader
                .read_sample(id, index)
                .map_err(|_| "sample_read_failed")?;
            hash_part(&mut hash, &sample.data);
        }
        Some(format!("{:x}", hash.finalize()))
    } else {
        None
    };
    // No Display/Debug of parser errors, metadata, handler names, or paths.
    // Values stay inside this function; only bounded booleans and store ID leave.
    let tag = mp4ameta::Tag::read_from(&mut Cursor::new(bytes))
        .map_err(|_| "metadata_invalid_or_unsupported")?;
    let purchase_date_tag_present = single_nonempty_text(&tag, *b"purd");
    let account_tag_present = single_nonempty_text(&tag, *b"apID");
    let store_song_id = song_id(&tag);
    let purchase_evidence = if entry == "aac"
        && purchase_date_tag_present
        && account_tag_present
        && store_song_id.is_some()
    {
        "matched_tag_pattern"
    } else {
        "missing_or_invalid"
    };
    Ok(Probe {
        parser: "mp4-track-0.1.0+mp4ameta-0.13.0-experiment",
        purchase_rule: "itunes-tag-pattern-v1",
        purchase_evidence,
        primary_entry: entry,
        samples,
        warning_count,
        purchase_date_tag_present,
        account_tag_present,
        store_song_id,
        audio_digest_scheme: digest.as_ref().map(|_| "rustune-aac-probe-v1"),
        audio_digest: digest,
    })
}

fn run() -> Result<(), &'static str> {
    let mut args = std::env::args_os().skip(1);
    let path = args
        .next()
        .ok_or("usage: rustune-audio-probe EXPLICIT_AUTHORIZED_FILE")?;
    if args.next().is_some() {
        return Err("one_file_required");
    }
    let file = std::fs::File::open(path).map_err(|_| "open_failed")?;
    let bytes = bounded_read(file)?;
    let result = probe(&bytes)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&result).map_err(|_| "serialization_failed")?
    );
    Ok(())
}

fn main() {
    if let Err(code) = run() {
        eprintln!("{code}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mp4_track::{AacConfig, Mp4Writer, SampleInput, TrackConfig, WriterConfig};
    use std::io::Write;

    // Synthetic packet bytes test container/digest behavior, NOT AAC decoding.
    fn fixture(packet: &[u8], handler: &str, codec: Option<Codec>) -> Vec<u8> {
        let mut writer = Mp4Writer::new(Cursor::new(Vec::new()), WriterConfig::default()).unwrap();
        let id = writer
            .add_track(TrackConfig {
                kind: TrackKind::Audio,
                timescale: 44100,
                language: "und".into(),
                handler_name: handler.into(),
                width: 0.0,
                height: 0.0,
                edit_list: vec![],
                codec: codec.unwrap_or(Codec::Aac {
                    config: AacConfig {
                        raw_esds: vec![
                            0, 0, 0, 0, 3, 25, 0, 1, 0, 4, 17, 0x40, 0x15, 0, 0, 0, 0, 0, 0, 0, 0,
                            0, 0, 0, 5, 2, 0x12, 0x10, 6, 1, 2,
                        ],
                        object_type: 0x40,
                        audio_specific_config: vec![0x12, 0x10],
                        channels: 2,
                        sample_rate: 44100,
                        sidecars: vec![],
                    },
                }),
            })
            .unwrap();
        writer
            .write_sample(
                id,
                &SampleInput {
                    data: packet,
                    duration: 1024,
                    cts_offset: 0,
                    is_sync: true,
                },
            )
            .unwrap();
        writer.finish().unwrap().into_inner()
    }

    #[test]
    fn synthetic_cd_like_aac_has_no_purchase_evidence() {
        let result = probe(&fixture(&[1, 2, 3], "synthetic", None)).unwrap();
        assert_eq!(result.primary_entry, "aac");
        assert_eq!(result.samples, 1);
        assert_eq!(result.purchase_evidence, "missing_or_invalid");
        assert!(!result.account_tag_present);
    }

    #[test]
    fn metadata_and_container_offsets_do_not_change_digest() {
        let a = probe(&fixture(&[1, 2, 3], "short", None)).unwrap();
        let b = probe(&fixture(&[1, 2, 3], "different handler size", None)).unwrap();
        assert_eq!(a.audio_digest, b.audio_digest);
        let c = probe(&fixture(&[1, 2, 4], "short", None)).unwrap();
        assert_ne!(a.audio_digest, c.audio_digest);
    }

    #[test]
    fn malformed_and_truncated_inputs_fail_with_static_errors() {
        assert!(probe(&[]).is_err());
        assert!(probe(&[0xff; 32]).is_err());
        let bytes = fixture(&[1, 2, 3], "synthetic", None);
        assert!(probe(&bytes[..bytes.len() - 1]).is_err());
    }

    #[test]
    fn digest_has_unambiguous_packet_boundaries() {
        let mut a = Sha256::new();
        hash_part(&mut a, &[1]);
        hash_part(&mut a, &[2, 3]);
        let mut b = Sha256::new();
        hash_part(&mut b, &[1, 2]);
        hash_part(&mut b, &[3]);
        assert_ne!(a.finalize(), b.finalize());
    }

    #[test]
    fn unsupported_entries_never_report_aac() {
        for code in [*b"enca", *b"drms", *b"alac"] {
            let bytes = fixture(
                &[1, 2, 3],
                "synthetic",
                Some(Codec::Other {
                    entry: mp4_track::FourCc(code),
                    raw: vec![0; 28],
                }),
            );
            if let Ok(result) = probe(&bytes) {
                assert_ne!(result.primary_entry, "aac");
                assert!(result.audio_digest.is_none());
                assert_eq!(result.purchase_evidence, "missing_or_invalid");
            }
        }
    }

    #[test]
    fn sample_outside_file_is_rejected() {
        let mut bytes = fixture(&[1, 2, 3], "synthetic", None);
        // This byte search only edits our generated fixture, never parses input.
        let at = bytes.windows(4).position(|part| part == b"stco").unwrap();
        bytes[at + 12..at + 16].copy_from_slice(&u32::MAX.to_be_bytes());
        assert!(probe(&bytes).is_err());
    }

    fn edit_tags(bytes: Vec<u8>, edit: impl FnOnce(&mut mp4ameta::Tag)) -> Vec<u8> {
        let mut memory = Cursor::new(bytes);
        let mut tag = mp4ameta::Tag::read_from(&mut memory).unwrap();
        edit(&mut tag);
        tag.write_to(&mut memory).unwrap();
        memory.into_inner()
    }

    fn add_synthetic_purchase_tags(tag: &mut mp4ameta::Tag) {
        tag.set_data(
            mp4ameta::Fourcc(*b"purd"),
            mp4ameta::Data::Utf8("2026-10-18 00:00:00".into()),
        );
        tag.set_data(
            mp4ameta::Fourcc(*b"apID"),
            mp4ameta::Data::Utf8("synthetic@example.invalid".into()),
        );
        tag.set_data(
            mp4ameta::Fourcc(*b"cnID"),
            mp4ameta::Data::BeSigned(123_u32.to_be_bytes().to_vec()),
        );
    }

    #[test]
    fn purchase_pattern_requires_all_tags_but_is_not_an_authenticity_proof() {
        let bytes = edit_tags(
            fixture(&[1, 2, 3], "synthetic", None),
            add_synthetic_purchase_tags,
        );
        assert_eq!(
            probe(&bytes).unwrap().purchase_evidence,
            "matched_tag_pattern"
        );
        for code in [*b"purd", *b"apID", *b"cnID"] {
            let missing = edit_tags(bytes.clone(), |tag| {
                tag.remove_data_of(&mp4ameta::Fourcc(code))
            });
            assert_eq!(
                probe(&missing).unwrap().purchase_evidence,
                "missing_or_invalid"
            );
        }
    }

    #[test]
    fn invalid_or_duplicate_purchase_tags_do_not_match() {
        let bytes = edit_tags(
            fixture(&[1, 2, 3], "synthetic", None),
            add_synthetic_purchase_tags,
        );
        for id in [vec![], vec![1, 2, 3], vec![0, 0, 0, 0], vec![0; 8]] {
            let invalid = edit_tags(bytes.clone(), |tag| {
                tag.set_data(mp4ameta::Fourcc(*b"cnID"), mp4ameta::Data::BeSigned(id))
            });
            assert_eq!(
                probe(&invalid).unwrap().purchase_evidence,
                "missing_or_invalid"
            );
        }
        for code in [*b"purd", *b"apID"] {
            let invalid = edit_tags(bytes.clone(), |tag| {
                tag.set_data(mp4ameta::Fourcc(code), mp4ameta::Data::Utf8("   ".into()))
            });
            assert_eq!(
                probe(&invalid).unwrap().purchase_evidence,
                "missing_or_invalid"
            );
        }
        let mut tag = mp4ameta::Tag::default();
        add_synthetic_purchase_tags(&mut tag);
        tag.add_data(
            mp4ameta::Fourcc(*b"apID"),
            mp4ameta::Data::Utf8("another@example.invalid".into()),
        );
        assert!(!single_nonempty_text(&tag, *b"apID"));
    }

    #[test]
    fn malformed_top_level_lengths_and_trailing_bytes_are_rejected() {
        let mut bytes = fixture(&[1, 2, 3], "synthetic", None);
        bytes.extend_from_slice(&[1, 2, 3]);
        assert!(matches!(probe(&bytes), Err("box_header_invalid")));
        assert!(media_ranges(&[0, 0, 0, 4, b'm', b'd', b'a', b't']).is_err());
        assert!(
            media_ranges(&[0, 0, 0, 1, b'm', b'd', b'a', b't', 0, 0, 0, 0, 0, 0, 0, 15]).is_err()
        );
    }

    #[test]
    fn tag_edits_preserve_identity_and_never_serialize_buyer_values() {
        let bytes = edit_tags(
            fixture(&[1, 2, 3], "synthetic", None),
            add_synthetic_purchase_tags,
        );
        let before = probe(&bytes).unwrap();
        let edited = edit_tags(bytes, |tag| {
            tag.set_title("changed title");
            tag.set_data(
                mp4ameta::Fourcc(*b"apID"),
                mp4ameta::Data::Utf8("private-test@example.invalid".into()),
            );
        });
        let after = probe(&edited).unwrap();
        assert_eq!(before.audio_digest, after.audio_digest);
        assert_eq!(before.store_song_id, after.store_song_id);
        let output = serde_json::to_string(&after).unwrap();
        assert!(!output.contains("private-test"));
        assert!(!output.contains("changed title"));
    }

    #[test]
    fn matching_titles_and_store_ids_do_not_overwrite_audio_difference() {
        let make = |packet| {
            edit_tags(fixture(packet, "synthetic", None), |tag| {
                add_synthetic_purchase_tags(tag);
                tag.set_title("identical title");
            })
        };
        let a = probe(&make(&[1, 2, 3])).unwrap();
        let b = probe(&make(&[4, 5, 6])).unwrap();
        assert_eq!(a.store_song_id, b.store_song_id);
        assert_ne!(a.audio_digest, b.audio_digest);
    }

    #[test]
    fn sample_inside_file_but_outside_mdat_is_rejected() {
        let mut bytes = fixture(&[1, 2, 3], "synthetic", None);
        let at = bytes.windows(4).position(|part| part == b"stco").unwrap();
        bytes[at + 12..at + 16].copy_from_slice(&8_u32.to_be_bytes());
        assert!(matches!(probe(&bytes), Err("sample_outside_media_data")));
    }

    #[test]
    #[ignore = "explicitly authorized local audio only; never run in CI"]
    fn authorized_local_audio_tag_and_copy_variants() {
        let path =
            std::env::var_os("RUSTUNE_AUDIO_TEST_FILE").expect("explicit local file required");
        let bytes =
            bounded_read(std::fs::File::open(path).expect("local file open failed")).unwrap();
        let before = probe(&bytes).unwrap();
        assert_eq!(before.primary_entry, "aac");
        assert_eq!(before.purchase_evidence, "matched_tag_pattern");
        // Private auto-deleted temporary copy, never a repository fixture.
        let mut temporary = tempfile::NamedTempFile::new().unwrap();
        temporary.write_all(&bytes).unwrap();
        let copy = bounded_read(temporary.reopen().unwrap()).unwrap();
        let duplicate = probe(&copy).unwrap();
        assert_eq!(before.audio_digest, duplicate.audio_digest);
        assert_eq!(before.store_song_id, duplicate.store_song_id);
        let edited = edit_tags(copy, |tag| tag.set_title("local in-memory metadata edit"));
        let after = probe(&edited).unwrap();
        assert_eq!(before.audio_digest, after.audio_digest);
        assert_eq!(before.store_song_id, after.store_song_id);
        assert_eq!(after.purchase_evidence, "matched_tag_pattern");
    }
}
