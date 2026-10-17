//! artworkの制限付きデコード・縮小・フォールバック・保存のテスト。画像はすべてテスト内で合成する。

mod support;

use std::fs;
use std::sync::Arc;

use image::codecs::jpeg::JpegEncoder;
use image::codecs::png::PngEncoder;
use image::{ExtendedColorType, ImageEncoder, RgbImage, RgbaImage};
use rustune_server::artwork::{
    self, ArtworkError, ArtworkFormat, ArtworkOrigin, EmbeddedArtwork, MAX_INPUT_BYTES,
    prepare_embedded, prepare_image,
};
use rustune_server::storage::{ManagedKind, Storage};
use support::FaultyIo;
use uuid::Uuid;

fn gradient(width: u32, height: u32) -> RgbImage {
    RgbImage::from_fn(width, height, |x, y| {
        image::Rgb([(x % 256) as u8, (y % 256) as u8, ((x + y) % 256) as u8])
    })
}

fn jpeg(width: u32, height: u32) -> Vec<u8> {
    let mut out = Vec::new();
    JpegEncoder::new_with_quality(&mut out, 90)
        .encode_image(&gradient(width, height))
        .unwrap();
    out
}

fn png_rgba(width: u32, height: u32) -> Vec<u8> {
    let image = RgbaImage::from_fn(width, height, |x, _| image::Rgba([x as u8, 10, 20, 128]));
    let mut out = Vec::new();
    PngEncoder::new(&mut out)
        .write_image(image.as_raw(), width, height, ExtendedColorType::Rgba8)
        .unwrap();
    out
}

fn decode_dims(bytes: &[u8]) -> (u32, u32) {
    let image = image::load_from_memory(bytes).unwrap();
    (image.width(), image.height())
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &byte in bytes {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

fn png_chunk(kind: &[u8; 4], data: &[u8]) -> Vec<u8> {
    let mut chunk = (data.len() as u32).to_be_bytes().to_vec();
    let mut body = kind.to_vec();
    body.extend_from_slice(data);
    chunk.extend_from_slice(&body);
    chunk.extend_from_slice(&crc32(&body).to_be_bytes());
    chunk
}

#[test]
fn jpeg_is_downscaled_for_display_and_thumbnail_keeping_aspect() {
    let staged = prepare_image(&jpeg(1000, 500), ArtworkOrigin::Embedded).unwrap();
    assert_eq!((staged.source_width, staged.source_height), (1000, 500));
    assert_eq!(staged.display.format, ArtworkFormat::Jpeg);
    assert_eq!((staged.display.width, staged.display.height), (512, 256));
    assert_eq!((staged.thumbnail.width, staged.thumbnail.height), (128, 64));
    assert_eq!(decode_dims(&staged.display.bytes), (512, 256));
    assert_eq!(decode_dims(&staged.thumbnail.bytes), (128, 64));
    assert_eq!(staged.digest.len(), 64);
}

#[test]
fn small_png_with_alpha_is_not_upscaled() {
    let staged = prepare_image(&png_rgba(64, 32), ArtworkOrigin::Embedded).unwrap();
    assert_eq!(staged.display.format, ArtworkFormat::Jpeg);
    assert_eq!(decode_dims(&staged.display.bytes), (64, 32));
    assert_eq!(decode_dims(&staged.thumbnail.bytes), (64, 32));

    let staged = prepare_image(&png_rgba(300, 40), ArtworkOrigin::Embedded).unwrap();
    assert_eq!(decode_dims(&staged.display.bytes), (300, 40));
    assert_eq!(decode_dims(&staged.thumbnail.bytes), (128, 17));
}

#[test]
fn reencoding_drops_exif_metadata() {
    let original = jpeg(40, 40);
    let mut payload = b"Exif\0\0".to_vec();
    payload.extend_from_slice(b"MM\0\x2a\0\0\0\x08\0\0 purchaser-like-secret");
    let mut with_exif = original[..2].to_vec();
    with_exif.extend_from_slice(&[0xFF, 0xE1]);
    with_exif.extend_from_slice(&((payload.len() + 2) as u16).to_be_bytes());
    with_exif.extend_from_slice(&payload);
    with_exif.extend_from_slice(&original[2..]);

    let staged = prepare_image(&with_exif, ArtworkOrigin::Embedded).unwrap();
    for bytes in [&staged.display.bytes, &staged.thumbnail.bytes] {
        assert!(!bytes.windows(4).any(|w| w == b"Exif"));
        assert!(!bytes.windows(6).any(|w| w == b"secret"));
    }
}

#[test]
fn enriched_image_without_transform_permission_keeps_provided_bytes() {
    let bytes = png_rgba(100, 100);
    let staged = prepare_image(
        &bytes,
        ArtworkOrigin::Enriched {
            transform_allowed: false,
        },
    )
    .unwrap();
    assert_eq!(staged.display.format, ArtworkFormat::Png);
    assert_eq!(staged.display.bytes, bytes);
    assert_eq!(staged.thumbnail.bytes, bytes);
}

#[test]
fn missing_image_is_absent_not_an_error() {
    assert!(matches!(prepare_embedded(None), EmbeddedArtwork::Absent));
    assert!(matches!(
        prepare_embedded(Some(&[])),
        EmbeddedArtwork::Absent
    ));
    assert!(matches!(
        prepare_embedded(Some(&jpeg(8, 8))),
        EmbeddedArtwork::Prepared(_)
    ));
}

#[test]
fn huge_png_dimensions_are_rejected_from_header_only() {
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&100_000u32.to_be_bytes());
    ihdr.extend_from_slice(&100_000u32.to_be_bytes());
    ihdr.extend_from_slice(&[8, 2, 0, 0, 0]);
    let mut bytes = b"\x89PNG\r\n\x1a\n".to_vec();
    bytes.extend(png_chunk(b"IHDR", &ihdr));
    bytes.extend(png_chunk(
        b"IDAT",
        &[0x78, 0x9c, 0x03, 0x00, 0x00, 0x00, 0x00, 0x01],
    ));
    bytes.extend(png_chunk(b"IEND", &[]));
    let error = prepare_image(&bytes, ArtworkOrigin::Embedded).unwrap_err();
    assert!(
        matches!(
            error,
            ArtworkError::DimensionsTooLarge {
                width: 100_000,
                height: 100_000
            }
        ),
        "{error:?}"
    );
    assert_eq!(error.warning_code(), "artwork_too_large");
}

#[test]
fn huge_jpeg_dimensions_are_rejected() {
    let mut bytes = jpeg(16, 16);
    let sof = bytes
        .windows(2)
        .position(|w| w == [0xFF, 0xC0])
        .expect("baseline SOF0");
    bytes[sof + 5..sof + 7].copy_from_slice(&60_000u16.to_be_bytes());
    bytes[sof + 7..sof + 9].copy_from_slice(&60_000u16.to_be_bytes());
    let error = prepare_image(&bytes, ArtworkOrigin::Embedded).unwrap_err();
    assert!(
        matches!(error, ArtworkError::DimensionsTooLarge { .. }),
        "{error:?}"
    );
}

#[test]
fn highly_compressed_png_bomb_is_rejected_before_allocating_pixels() {
    // 4100x4100 = 16.81MP（上限16MPを超える）の単色画像は数十KBに圧縮される。
    let side = 4100u32;
    let pixels = vec![0u8; (side * side) as usize];
    let mut bomb = Vec::new();
    PngEncoder::new(&mut bomb)
        .write_image(&pixels, side, side, ExtendedColorType::L8)
        .unwrap();
    drop(pixels);
    assert!(bomb.len() < 200 * 1024, "{}", bomb.len());
    let error = prepare_image(&bomb, ArtworkOrigin::Embedded).unwrap_err();
    assert!(
        matches!(error, ArtworkError::DimensionsTooLarge { .. }),
        "{error:?}"
    );
}

#[test]
fn oversized_input_and_unsupported_formats_are_rejected() {
    let mut big = jpeg(8, 8);
    big.resize(MAX_INPUT_BYTES + 1, 0);
    assert!(matches!(
        prepare_image(&big, ArtworkOrigin::Embedded),
        Err(ArtworkError::InputTooLarge { .. })
    ));
    let error = prepare_image(b"GIF89a\x01\x00\x01\x00", ArtworkOrigin::Embedded).unwrap_err();
    assert!(matches!(error, ArtworkError::UnsupportedFormat));
    assert_eq!(error.warning_code(), "artwork_unsupported");
}

#[test]
fn corrupt_images_fall_back_with_warning() {
    let png = png_rgba(32, 32);
    let truncated_png = &png[..png.len() / 2];
    let mut bad_crc_png = png.clone();
    let idat = bad_crc_png.windows(4).position(|w| w == b"IDAT").unwrap();
    bad_crc_png[idat + 6] ^= 0xFF;
    let jpeg_header_only = &jpeg(32, 32)[..20];
    let garbage_jpeg = [0xFF, 0xD8, 0xFF, 0x00, 0x13, 0x37, 0x00, 0x00];

    for bytes in [
        truncated_png,
        &bad_crc_png[..],
        jpeg_header_only,
        &garbage_jpeg[..],
    ] {
        match prepare_embedded(Some(bytes)) {
            EmbeddedArtwork::Fallback(error) => {
                assert_eq!(error.warning_code(), "artwork_invalid", "{error:?}")
            }
            other => panic!("corrupt image accepted: {other:?}"),
        }
    }
}

#[test]
fn store_promotes_both_variants_and_remove_cleans_up() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(dir.path()).unwrap();
    let staged = prepare_image(&jpeg(600, 600), ArtworkOrigin::Embedded).unwrap();
    let id = Uuid::new_v4();
    let stored = artwork::store(&storage, id, &staged).unwrap();
    assert_eq!(stored.display.relative_path(), format!("artwork/{id}.jpg"));
    assert_eq!(
        stored.thumbnail.relative_path(),
        format!("artwork/{id}.thumb.jpg")
    );
    assert_eq!(
        fs::read(storage.path_of(stored.display)).unwrap(),
        staged.display.bytes
    );
    assert_eq!(storage.list_managed(ManagedKind::Artwork).unwrap().len(), 2);
    assert!(
        storage
            .list_managed(ManagedKind::Staging)
            .unwrap()
            .is_empty()
    );
    // 同じIDへは上書きしない。
    assert!(artwork::store(&storage, id, &staged).is_err());

    artwork::remove(&storage, &stored).unwrap();
    assert!(
        storage
            .list_managed(ManagedKind::Artwork)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn store_failure_leaves_no_partial_artwork() {
    let dir = tempfile::tempdir().unwrap();
    let io = Arc::new(FaultyIo::default());
    let storage = Storage::with_io(dir.path(), io.clone()).unwrap();
    let staged = prepare_image(&jpeg(200, 200), ArtworkOrigin::Embedded).unwrap();
    // 表示用のrenameは成功し、一覧用で失敗する。
    *io.renames_before_failure.lock().unwrap() = Some(1);
    assert!(artwork::store(&storage, Uuid::new_v4(), &staged).is_err());
    assert!(
        storage
            .list_managed(ManagedKind::Artwork)
            .unwrap()
            .is_empty()
    );
    assert!(
        storage
            .list_managed(ManagedKind::Staging)
            .unwrap()
            .is_empty()
    );
}
