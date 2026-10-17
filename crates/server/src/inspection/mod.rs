//! 購入AACの上限付き解析。購入者タグの値はこのモジュールの外へ出さない。
//!
//! 処理順はMODULE_DESIGN §7に従う：ファイル長とbox境界（mp4-atom Header）→
//! トラック・sample entry・AAC設定・sample参照（mp4-track）→ 購入由来タグ（mp4ameta）→
//! 許可メタデータと画像候補 → 音声digest。購入規則に一致しない入力のsampleは読まない。
mod bounds;
use mp4_track::{Codec, Mp4Reader, TrackKind};
use rustune_contracts::error::ErrorCode;
use rustune_domain::{
    identity::{AUDIO_DIGEST_SCHEME, AudioDigest, VerifiedStoreId},
    metadata::{Metadata, Text, TrackNumber},
    track::{SampleRange, ValidatedAudio},
};
use sha2::{Digest, Sha256};
use std::io::{Read, Seek, SeekFrom};

pub const PARSER_VERSION: &str =
    "rustune-inspection-v1/mp4-track-0.1.0/mp4ameta-0.13.0/itunes-tag-pattern-v1";
#[derive(Clone, Copy, Debug)]
pub struct InspectionLimits {
    pub max_file_bytes: u64,
    pub max_metadata_bytes: u64,
    pub max_sample_bytes: u32,
    pub max_samples: u32,
    pub max_boxes: u32,
    pub max_depth: u8,
}
impl Default for InspectionLimits {
    fn default() -> Self {
        Self {
            max_file_bytes: 200 * 1024 * 1024,
            max_metadata_bytes: 16 * 1024 * 1024,
            max_sample_bytes: 1024 * 1024,
            max_samples: 1_000_000,
            max_boxes: 100_000,
            max_depth: 32,
        }
    }
}
/// 除外・保留の内部理由。生のパーサー/OSエラーは保持しない。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reason {
    /// 読み取り自体の失敗（一時的な可能性がある）。
    ReadFailed,
    /// box長・EOF・sample参照などの構造不正、または途中までのファイル。
    InvalidContainer,
    /// AAC以外のcodec、fragmented MP4、複数音声トラック等の未対応形式。
    UnsupportedCodec,
    /// `itunes-tag-pattern-v1`に一致しない。
    PurchaseUnverified,
    /// 解析上限（サイズ・box数・深さ・sample数等）を超えた。
    AnalysisLimit,
    /// 解析の前後でハンドルの属性が変化した。
    SourceChanged,
}
impl Reason {
    /// 公開契約のcodeへ変換する。解析上限超過は契約に専用codeがないため
    /// `unsupported_codec`（対応外の音源）として扱う。
    pub fn error_code(self) -> ErrorCode {
        match self {
            Self::ReadFailed => ErrorCode::IoFailure,
            Self::InvalidContainer => ErrorCode::CorruptAudio,
            Self::UnsupportedCodec | Self::AnalysisLimit => ErrorCode::UnsupportedCodec,
            Self::PurchaseUnverified => ErrorCode::PurchaseUnverified,
            Self::SourceChanged => ErrorCode::FileUnstable,
        }
    }
    /// 内容が変わるまで自動再試行しない理由はfalse。
    pub fn retryable(self) -> bool {
        matches!(
            self,
            Self::ReadFailed | Self::SourceChanged | Self::InvalidContainer
        )
    }
}
#[derive(Debug)]
pub struct InspectedAudio {
    pub audio: ValidatedAudio,
    pub metadata: Metadata,
    pub artwork: Vec<ArtworkCandidate>,
    pub parser_version: &'static str,
}
/// 埋め込み画像の符号化バイト列のみ。形式・寸法の検証はartworkで行う。
#[derive(Debug)]
pub struct ArtworkCandidate {
    pub bytes: Vec<u8>,
}
#[derive(Debug)]
pub enum InspectionResult {
    Accepted(Box<InspectedAudio>),
    Deferred(Reason),
    Rejected(Reason),
}

/// 実ファイルにはこの境界を使う。解析の前後でハンドル属性を照合し、
/// 書き込み中のファイルから古い検証結果を返さない。
pub fn inspect_source(
    source: &mut crate::source::OpenedSource,
    limits: InspectionLimits,
) -> InspectionResult {
    if source.verify_unchanged().is_err() {
        return InspectionResult::Deferred(Reason::SourceChanged);
    }
    let result = inspect(source.reader(), limits);
    if source.verify_unchanged().is_err() {
        return InspectionResult::Deferred(Reason::SourceChanged);
    }
    result
}
/// 任意のRead+Seekを解析する。`Deferred`は次回の走査で再評価する。
pub fn inspect<R: Read + Seek>(reader: &mut R, limits: InspectionLimits) -> InspectionResult {
    match inspect_inner(reader, limits) {
        Ok(audio) => InspectionResult::Accepted(Box::new(audio)),
        Err(Reason::ReadFailed) => InspectionResult::Deferred(Reason::ReadFailed),
        Err(reason) => InspectionResult::Rejected(reason),
    }
}
fn inspect_inner<R: Read + Seek>(
    input: &mut R,
    limits: InspectionLimits,
) -> Result<InspectedAudio, Reason> {
    let size = input
        .seek(SeekFrom::End(0))
        .map_err(|_| Reason::ReadFailed)?;
    if size == 0 {
        return Err(Reason::InvalidContainer);
    }
    if size > limits.max_file_bytes {
        return Err(Reason::AnalysisLimit);
    }
    let mut bounded = bounds::Bounded::new(input, size).map_err(|_| Reason::ReadFailed)?;
    let layout = bounds::validate(&mut bounded, size, limits)?;
    let mut parser_limits = mp4_track::Limits::default();
    parser_limits.max_moov_bytes = limits.max_metadata_bytes;
    parser_limits.max_moof_bytes = 0;
    parser_limits.max_sample_bytes = limits.max_sample_bytes;
    parser_limits.max_sample_count = limits.max_samples;
    parser_limits.max_fragments = 0;
    parser_limits.max_depth = limits.max_depth;
    bounded
        .seek(SeekFrom::Start(0))
        .map_err(|_| Reason::ReadFailed)?;
    bounded.masks = layout.movie_user_data.clone();
    let parser = Mp4Reader::open_with(&mut bounded, parser_limits).map_err(parser_error)?;
    if parser.is_fragmented() {
        return Err(Reason::UnsupportedCodec);
    }
    let tracks: Vec<_> = parser
        .tracks()
        .iter()
        .filter(|t| t.kind == TrackKind::Audio)
        .collect();
    if tracks.len() != 1 {
        return Err(Reason::UnsupportedCodec);
    }
    let track = tracks[0];
    let Codec::Aac { config } = &track.codec else {
        return Err(Reason::UnsupportedCodec);
    };
    if config.object_type != 0x40 || config.audio_specific_config.is_empty() {
        return Err(Reason::UnsupportedCodec);
    }
    if track.sample_count == 0 || track.timescale == 0 {
        return Err(Reason::InvalidContainer);
    }
    let duration = u128::from(track.duration) * 1000 / u128::from(track.timescale);
    let duration = u64::try_from(duration).map_err(|_| Reason::InvalidContainer)?;
    let (id, rate, channels, asc, count) = (
        track.id,
        config.sample_rate,
        config.channels,
        config.audio_specific_config.clone(),
        track.sample_count,
    );
    let mut samples = Vec::with_capacity(count as usize);
    let mut total_sample_bytes = 0_u64;
    for sample in parser.samples(id).map_err(parser_error)? {
        let end = sample
            .offset
            .checked_add(u64::from(sample.size))
            .ok_or(Reason::InvalidContainer)?;
        if sample.description_index != 1 {
            return Err(Reason::UnsupportedCodec);
        }
        if sample.size > limits.max_sample_bytes {
            return Err(Reason::AnalysisLimit);
        }
        let index = layout.media.partition_point(|r| r.start <= sample.offset);
        if sample.size == 0 || index == 0 || end > layout.media[index - 1].end {
            return Err(Reason::InvalidContainer);
        }
        total_sample_bytes = total_sample_bytes
            .checked_add(u64::from(sample.size))
            .ok_or(Reason::AnalysisLimit)?;
        if total_sample_bytes > size {
            return Err(Reason::AnalysisLimit);
        }
        if samples.len() >= count as usize {
            return Err(Reason::InvalidContainer);
        }
        samples.push(SampleRange {
            offset: sample.offset,
            length: u64::from(sample.size),
        });
    }
    if samples.len() != count as usize {
        return Err(Reason::InvalidContainer);
    }
    drop(parser);
    bounded.masks.clear();
    bounded
        .seek(SeekFrom::Start(0))
        .map_err(|_| Reason::ReadFailed)?;
    // mp4ameta はタグ読み取り専用。購入判定は下の規則で行い、値は外へ出さない。
    let cfg = mp4ameta::ReadConfig {
        read_meta_items: true,
        read_image_data: true,
        ..mp4ameta::ReadConfig::NONE
    };
    // 構造検査は通過済み。タグを読めない場合は購入由来を確認できない扱いにする。
    let tag =
        mp4ameta::Tag::read_with(&mut bounded, &cfg).map_err(|_| Reason::PurchaseUnverified)?;
    let store_id = purchase_id(&tag).ok_or(Reason::PurchaseUnverified)?;
    let metadata = Metadata {
        title: tag.title().and_then(|s| Text::new(s).ok()),
        artist: tag.artist().and_then(|s| Text::new(s).ok()),
        album: tag.album().and_then(|s| Text::new(s).ok()),
        album_artist: tag.album_artist().and_then(|s| Text::new(s).ok()),
        disc_number: tag.disc_number().and_then(|n| TrackNumber::new(n).ok()),
        track_number: tag.track_number().and_then(|n| TrackNumber::new(n).ok()),
    };
    let artwork = tag
        .artworks()
        .take(1)
        .map(|img| ArtworkCandidate {
            bytes: img.data.to_vec(),
        })
        .collect();
    // 購入規則の一致後に、検証済みのmdat内範囲だけを読み直してdigestを計算する。
    let mut hash = Sha256::new();
    hash.update(AUDIO_DIGEST_SCHEME.as_bytes());
    hash.update([0]);
    hash_part(&mut hash, &asc);
    let mut buffer = Vec::new();
    for sample in &samples {
        buffer.resize(sample.length as usize, 0);
        bounded
            .seek(SeekFrom::Start(sample.offset))
            .map_err(|_| Reason::ReadFailed)?;
        bounded
            .read_exact(&mut buffer)
            .map_err(|_| Reason::InvalidContainer)?;
        hash_part(&mut hash, &buffer);
    }
    let audio = ValidatedAudio::new(
        size,
        duration,
        rate,
        channels,
        asc,
        samples,
        AudioDigest::new(hash.finalize().into()),
        store_id,
    )
    .map_err(|_| Reason::InvalidContainer)?;
    Ok(InspectedAudio {
        audio,
        metadata,
        artwork,
        parser_version: PARSER_VERSION,
    })
}
fn hash_part(hash: &mut Sha256, value: &[u8]) {
    hash.update((value.len() as u64).to_be_bytes());
    hash.update(value);
}
fn parser_error(error: mp4_track::Error) -> Reason {
    match error {
        mp4_track::Error::LimitExceeded { .. } => Reason::AnalysisLimit,
        mp4_track::Error::Unsupported(_) => Reason::UnsupportedCodec,
        mp4_track::Error::Io(_) => Reason::ReadFailed,
        _ => Reason::InvalidContainer,
    }
}
/// `itunes-tag-pattern-v1`：purd・apIDが各1個の非空文字列（1024 bytes以下）、
/// cnIDが1個の4-byte big-endian非zero。返すのは数値のstore IDだけ。
fn purchase_id(tag: &mp4ameta::Tag) -> Option<VerifiedStoreId> {
    for code in [*b"purd", *b"apID"] {
        let ident = mp4ameta::Fourcc(code);
        let mut values = tag.data_of(&ident);
        let text = values.next()?.string()?;
        if text.trim().is_empty() || text.len() > 1024 || values.next().is_some() {
            return None;
        }
    }
    let ident = mp4ameta::Fourcc(*b"cnID");
    let mut values = tag.data_of(&ident);
    let id = u32::from_be_bytes(values.next()?.bytes()?.try_into().ok()?);
    if values.next().is_some() {
        return None;
    }
    VerifiedStoreId::from_matched_tag_pattern(id)
}

#[cfg(test)]
mod tests;
