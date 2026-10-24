use super::{InspectionLimits, Reason};
use mp4_atom::{FourCC, Header, ReadFrom};
use std::io::{Read, Seek, SeekFrom};
use std::ops::Range;

pub(super) struct Layout {
    pub media: Vec<Range<u64>>,
    /// moov直下のudtaのfourcc位置。mp4-trackへは`free`として見せる。
    pub movie_user_data: Vec<u64>,
}

/// 各パーサーへ渡すseek/readを解析開始時のファイル長内に制限する。
///
/// `masks`の各位置にある4 bytesは読み出し時に`free`へ置き換える。mp4-trackは
/// moov/udtaのタグを厳格に解析し、QuickTime形式metaや複数data（複数ジャケット等）を
/// 構造エラーにするため、タグはmp4ameta だけに読ませる。元ファイルは変更しない。
pub(super) struct Bounded<'a, R> {
    pub inner: &'a mut R,
    pub size: u64,
    position: u64,
    pub masks: Vec<u64>,
}
impl<'a, R: Read + Seek> Bounded<'a, R> {
    pub fn new(inner: &'a mut R, size: u64) -> std::io::Result<Self> {
        inner.seek(SeekFrom::Start(0))?;
        Ok(Self {
            inner,
            size,
            position: 0,
            masks: Vec::new(),
        })
    }
}
impl<R: Read> Read for Bounded<'_, R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let remaining = self
            .size
            .saturating_sub(self.position)
            .min(buf.len() as u64) as usize;
        let count = self.inner.read(&mut buf[..remaining])?;
        let read = self.position..self.position + count as u64;
        for &mask in &self.masks {
            for (index, byte) in b"free".iter().enumerate() {
                let at = mask + index as u64;
                if read.contains(&at) {
                    buf[(at - read.start) as usize] = *byte;
                }
            }
        }
        self.position = read.end;
        Ok(count)
    }
}
impl<R: Seek> Seek for Bounded<'_, R> {
    fn seek(&mut self, from: SeekFrom) -> std::io::Result<u64> {
        let next = match from {
            SeekFrom::Start(value) => i128::from(value),
            SeekFrom::End(delta) => i128::from(self.size) + i128::from(delta),
            SeekFrom::Current(delta) => i128::from(self.position) + i128::from(delta),
        };
        if next < 0 || next > i128::from(self.size) {
            return Err(std::io::Error::from(std::io::ErrorKind::InvalidInput));
        }
        let position = self.inner.seek(SeekFrom::Start(next as u64))?;
        self.position = position;
        Ok(position)
    }
}

/// `meta`はISO形式ではversion/flagsの4 bytesを持ち、QuickTime形式では直後に子boxが続く。
/// 先頭の子が`hdlr`である位置で判別する。
fn meta_prefix<R: Read + Seek>(reader: &mut R, body: u64, end: u64) -> Result<u64, Reason> {
    if end - body < 8 {
        return Ok(4.min(end - body));
    }
    reader
        .seek(SeekFrom::Start(body))
        .map_err(|_| Reason::ReadFailed)?;
    let mut head = [0; 8];
    reader
        .read_exact(&mut head)
        .map_err(|_| Reason::InvalidContainer)?;
    Ok(if &head[4..8] == b"hdlr" { 0 } else { 4 })
}

pub(super) fn validate<R: Read + Seek>(
    reader: &mut R,
    size: u64,
    limits: InspectionLimits,
) -> Result<Layout, Reason> {
    let mut layout = Layout {
        media: vec![],
        movie_user_data: vec![],
    };
    let mut budget = limits.max_boxes;
    walk(
        reader,
        0..size,
        *b"root",
        0,
        &mut budget,
        limits,
        &mut layout,
    )?;
    if layout.media.is_empty() {
        return Err(Reason::InvalidContainer);
    }
    Ok(layout)
}

/// 全階層で共有するbox数予算・深さ・親範囲内の境界だけをここで検査する。
/// header長のデコードはmp4-atom、AAC記述とsample tableはmp4-track、タグはmp4ameta が担う。
#[allow(clippy::too_many_arguments)]
fn walk<R: Read + Seek>(
    reader: &mut R,
    range: Range<u64>,
    parent: [u8; 4],
    depth: u8,
    budget: &mut u32,
    limits: InspectionLimits,
    layout: &mut Layout,
) -> Result<(), Reason> {
    if depth > limits.max_depth {
        return Err(Reason::AnalysisLimit);
    }
    let mut at = range.start;
    while at < range.end {
        *budget = budget.checked_sub(1).ok_or(Reason::AnalysisLimit)?;
        reader
            .seek(SeekFrom::Start(at))
            .map_err(|_| Reason::ReadFailed)?;
        if range.end - at < 8 {
            return Err(Reason::InvalidContainer);
        }
        let header = Header::read_from(reader).map_err(|_| Reason::InvalidContainer)?;
        let body = reader.stream_position().map_err(|_| Reason::ReadFailed)?;
        let end = header
            .size
            .map(|s| body.checked_add(s as u64).ok_or(Reason::InvalidContainer))
            .transpose()?
            .unwrap_or(range.end);
        if body > range.end || end > range.end || end < body {
            return Err(Reason::InvalidContainer);
        }
        let kind: [u8; 4] = header.kind.into();
        if kind == *b"moof" || kind == *b"mvex" {
            return Err(Reason::UnsupportedCodec);
        }
        if kind == *b"moov" && end - at > limits.max_metadata_bytes {
            return Err(Reason::AnalysisLimit);
        }
        if parent == *b"root" && header.kind == FourCC::new(b"mdat") {
            layout.media.push(body..end);
        }
        if parent == *b"moov" && kind == *b"udta" {
            // header長に関わらずfourccはbox先頭から4 bytes目にある。
            layout.movie_user_data.push(at + 4);
        }
        let prefix = match &kind {
            b"moov" | b"trak" | b"mdia" | b"minf" | b"stbl" | b"udta" | b"edts" | b"dinf"
            | b"ilst" => Some(0),
            b"meta" => Some(meta_prefix(reader, body, end)?),
            b"stsd" | b"dref" => Some(8),
            b"mp4a" => {
                // version 0のaudio sample entryは子boxの前に28 bytesの固定領域を持つ。
                if end - body < 28 {
                    return Err(Reason::InvalidContainer);
                }
                reader
                    .seek(SeekFrom::Start(body + 8))
                    .map_err(|_| Reason::ReadFailed)?;
                let mut version = [0; 2];
                reader
                    .read_exact(&mut version)
                    .map_err(|_| Reason::InvalidContainer)?;
                if version != [0, 0] {
                    return Err(Reason::UnsupportedCodec);
                }
                Some(28)
            }
            _ if parent == *b"ilst" => Some(0),
            _ => None,
        };
        if let Some(prefix) = prefix {
            let start = body.checked_add(prefix).ok_or(Reason::InvalidContainer)?;
            if start > end {
                return Err(Reason::InvalidContainer);
            }
            walk(
                reader,
                start..end,
                kind,
                depth.checked_add(1).ok_or(Reason::AnalysisLimit)?,
                budget,
                limits,
                layout,
            )?;
        }
        at = end;
    }
    Ok(())
}
