//! ジャケット画像の制限付きデコード・サムネイル生成・保存。
//!
//! - 入力はJPEG/PNGに限定し、入力10 MiB・展開16メガピクセルを上限とする。
//!   寸法はヘッダーだけを読んだ段階で検査し、画素バッファを確保する前に拒否する。
//! - 同時に処理する画像は1枚（プロセス内のロックで直列化）。
//! - 表示用は最大512px、一覧用は最大128px。元より拡大しない。
//! - 再エンコードは画素からJPEGを作り直すため、EXIF等の付随情報を持ち越さない。
//! - 埋め込み画像が壊れていても音源の取り込みは続け、警告と既定画像へ
//!   フォールバックする（[`prepare_embedded`]）。
//! - 外部画像の取得は扱わない（タスク13）。

use std::fmt;
use std::io::Cursor;
use std::sync::Mutex;

use image::codecs::jpeg::JpegEncoder;
use image::imageops::FilterType;
use image::{DynamicImage, ImageDecoder, ImageFormat, ImageReader, Limits};
use sha2::{Digest, Sha256};
use uuid::Uuid;

pub use crate::storage::{ArtworkFormat, ArtworkVariant};
use crate::storage::{ManagedFile, Storage, StorageError};

/// 入力バイト列の上限（10 MiB）。
pub const MAX_INPUT_BYTES: usize = 10 * 1024 * 1024;
/// 展開後の画素数の上限（16メガピクセル）。
pub const MAX_PIXELS: u64 = 16_000_000;
/// デコーダが確保できるメモリの上限。16MPの16bit RGBAを収める程度。
pub const MAX_DECODE_ALLOC: u64 = 160 * 1024 * 1024;
/// 表示用の最大辺。
pub const DISPLAY_MAX_EDGE: u32 = 512;
/// 一覧用の最大辺。
pub const THUMBNAIL_MAX_EDGE: u32 = 128;
const JPEG_QUALITY: u8 = 85;

static DECODE_LOCK: Mutex<()> = Mutex::new(());

/// 画像の由来。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArtworkOrigin {
    /// 音源に埋め込まれた画像。常に再エンコード・縮小する。
    Embedded,
    /// 補完APIの画像。提供元の条件が加工を認めない場合は`transform_allowed=false`とし、
    /// デコード検証だけ行って提供されたバイト列のまま保持する。
    Enriched { transform_allowed: bool },
}

/// 保存用にエンコード済みの画像1枚。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EncodedImage {
    pub format: ArtworkFormat,
    pub width: u32,
    pub height: u32,
    pub bytes: Vec<u8>,
}

/// 検証・縮小済みでまだ保存していないジャケット。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StagedArtwork {
    pub origin: ArtworkOrigin,
    pub source_width: u32,
    pub source_height: u32,
    pub display: EncodedImage,
    pub thumbnail: EncodedImage,
    /// 表示用バイト列のSHA-256（16進）。artworks.digestに保存する。
    pub digest: String,
}

/// 画像を使えない理由。音源の取り込みは止めず、画像警告として記録する。
#[derive(Debug)]
pub enum ArtworkError {
    InputTooLarge { bytes: usize },
    UnsupportedFormat,
    DimensionsTooLarge { width: u32, height: u32 },
    Corrupt(String),
    Encode(String),
}

impl ArtworkError {
    /// 取り込み項目に残す画像警告code。
    pub fn warning_code(&self) -> &'static str {
        match self {
            Self::InputTooLarge { .. } | Self::DimensionsTooLarge { .. } => "artwork_too_large",
            Self::UnsupportedFormat => "artwork_unsupported",
            Self::Corrupt(_) | Self::Encode(_) => "artwork_invalid",
        }
    }
}

impl fmt::Display for ArtworkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InputTooLarge { bytes } => write!(f, "画像データが大きすぎます（{bytes}バイト）"),
            Self::UnsupportedFormat => f.write_str("JPEG/PNG以外の画像です"),
            Self::DimensionsTooLarge { width, height } => {
                write!(f, "画像の寸法が大きすぎます（{width}x{height}）")
            }
            Self::Corrupt(detail) => write!(f, "画像を読み取れません: {detail}"),
            Self::Encode(detail) => write!(f, "画像を変換できません: {detail}"),
        }
    }
}

impl std::error::Error for ArtworkError {}

/// 先頭のマジックナンバーで形式を判定する。拡張子・申告形式は信用しない。
pub fn sniff_format(bytes: &[u8]) -> Option<ArtworkFormat> {
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some(ArtworkFormat::Jpeg)
    } else if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some(ArtworkFormat::Png)
    } else {
        None
    }
}

/// 画像を検証し、表示用・一覧用を用意する。
pub fn prepare_image(bytes: &[u8], origin: ArtworkOrigin) -> Result<StagedArtwork, ArtworkError> {
    if bytes.len() > MAX_INPUT_BYTES {
        return Err(ArtworkError::InputTooLarge { bytes: bytes.len() });
    }
    let format = sniff_format(bytes).ok_or(ArtworkError::UnsupportedFormat)?;
    let _guard = DECODE_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let image = decode_limited(bytes, format)?;
    let (source_width, source_height) = (image.width(), image.height());

    let transform = match origin {
        ArtworkOrigin::Embedded => true,
        ArtworkOrigin::Enriched { transform_allowed } => transform_allowed,
    };
    let (display, thumbnail) = if transform {
        let display_image = fit_within(&image, DISPLAY_MAX_EDGE);
        drop(image);
        let thumbnail_image = fit_within(&display_image, THUMBNAIL_MAX_EDGE);
        (encode_jpeg(&display_image)?, encode_jpeg(&thumbnail_image)?)
    } else {
        let original = EncodedImage {
            format,
            width: source_width,
            height: source_height,
            bytes: bytes.to_vec(),
        };
        (original.clone(), original)
    };
    let digest = Sha256::digest(&display.bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    Ok(StagedArtwork {
        origin,
        source_width,
        source_height,
        display,
        thumbnail,
        digest,
    })
}

fn decode_limited(bytes: &[u8], format: ArtworkFormat) -> Result<DynamicImage, ArtworkError> {
    let format = match format {
        ArtworkFormat::Jpeg => ImageFormat::Jpeg,
        ArtworkFormat::Png => ImageFormat::Png,
    };
    let mut limits = Limits::default();
    limits.max_image_width = Some(u32::MAX);
    limits.max_image_height = Some(u32::MAX);
    limits.max_alloc = Some(MAX_DECODE_ALLOC);
    let mut reader = ImageReader::with_format(Cursor::new(bytes), format);
    reader.limits(limits);
    let decoder = reader
        .into_decoder()
        .map_err(|error| ArtworkError::Corrupt(error.to_string()))?;
    let (width, height) = decoder.dimensions();
    if width == 0 || height == 0 || u64::from(width) * u64::from(height) > MAX_PIXELS {
        return Err(ArtworkError::DimensionsTooLarge { width, height });
    }
    DynamicImage::from_decoder(decoder).map_err(|error| ArtworkError::Corrupt(error.to_string()))
}

/// 縦横比を保って最大辺`max_edge`へ縮小する。元が小さければそのまま。
fn fit_within(image: &DynamicImage, max_edge: u32) -> DynamicImage {
    if image.width() <= max_edge && image.height() <= max_edge {
        image.clone()
    } else {
        image.resize(max_edge, max_edge, FilterType::Lanczos3)
    }
}

fn encode_jpeg(image: &DynamicImage) -> Result<EncodedImage, ArtworkError> {
    let rgb = image.to_rgb8();
    let mut bytes = Vec::new();
    JpegEncoder::new_with_quality(&mut bytes, JPEG_QUALITY)
        .encode_image(&rgb)
        .map_err(|error| ArtworkError::Encode(error.to_string()))?;
    Ok(EncodedImage {
        format: ArtworkFormat::Jpeg,
        width: rgb.width(),
        height: rgb.height(),
        bytes,
    })
}

/// 音源の埋め込み画像の扱い。いずれの場合も音源の取り込みは続ける。
#[derive(Debug)]
pub enum EmbeddedArtwork {
    /// 画像なし。既定画像で表示する。
    Absent,
    Prepared(StagedArtwork),
    /// 画像はあるが使えない。警告を残し既定画像で表示する。
    Fallback(ArtworkError),
}

pub fn prepare_embedded(candidate: Option<&[u8]>) -> EmbeddedArtwork {
    match candidate {
        None | Some([]) => EmbeddedArtwork::Absent,
        Some(bytes) => match prepare_image(bytes, ArtworkOrigin::Embedded) {
            Ok(staged) => EmbeddedArtwork::Prepared(staged),
            Err(error) => EmbeddedArtwork::Fallback(error),
        },
    }
}

/// 保存済みジャケット。DBのartworks行に記録する値。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredArtwork {
    pub id: Uuid,
    pub origin: ArtworkOrigin,
    pub display: ManagedFile,
    pub thumbnail: ManagedFile,
    pub width: u32,
    pub height: u32,
    pub digest: String,
}

/// 表示用・一覧用を管理下の`artwork/`へ確定する。trackのready前に呼ぶ。
/// 途中で失敗した場合は書き込んだ分を削除してエラーを返す。呼び出し側は
/// 既定画像（artwork_idなし）へフォールバックする。
pub fn store(
    storage: &Storage,
    id: Uuid,
    artwork: &StagedArtwork,
) -> Result<StoredArtwork, StorageError> {
    if id.is_nil() {
        return Err(StorageError::InvalidTarget);
    }
    let display = ManagedFile::Artwork {
        id,
        variant: ArtworkVariant::Display,
        format: artwork.display.format,
    };
    let thumbnail = ManagedFile::Artwork {
        id,
        variant: ArtworkVariant::Thumbnail,
        format: artwork.thumbnail.format,
    };
    storage.install_bytes(display, &artwork.display.bytes)?;
    if let Err(error) = storage.install_bytes(thumbnail, &artwork.thumbnail.bytes) {
        let _ = storage.remove_managed(display);
        return Err(error);
    }
    Ok(StoredArtwork {
        id,
        origin: artwork.origin,
        display,
        thumbnail,
        width: artwork.display.width,
        height: artwork.display.height,
        digest: artwork.digest.clone(),
    })
}

/// DB参照がなくなった画像を削除する。参照の有無は呼び出し側（DB）が判定する。
pub fn remove(storage: &Storage, stored: &StoredArtwork) -> Result<(), StorageError> {
    storage.remove_managed(stored.thumbnail)?;
    storage.remove_managed(stored.display)
}
