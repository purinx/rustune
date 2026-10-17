//! コピー元の狭い入力境界。
//!
//! storageは取り込み元のパスを自由にopenしない。sourceモジュール（タスク06）が
//! 安全に開いた読み取りハンドルを[`CopySource`]として渡し、storageはその
//! ハンドルの属性とroot側エントリの属性をコピー前後に照合する。

use std::fs::{File, Metadata};
use std::io::{self, Read, Seek, SeekFrom};
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;

/// 元ファイルの同一性と内容変化を判定する属性。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceSnapshot {
    pub device: u64,
    pub inode: u64,
    pub size: u64,
    pub mtime_sec: i64,
    pub mtime_nsec: i64,
    pub ctime_sec: i64,
    pub ctime_nsec: i64,
}

impl SourceSnapshot {
    pub fn from_metadata(metadata: &Metadata) -> Self {
        Self {
            device: metadata.dev(),
            inode: metadata.ino(),
            size: metadata.size(),
            mtime_sec: metadata.mtime(),
            mtime_nsec: metadata.mtime_nsec(),
            ctime_sec: metadata.ctime(),
            ctime_nsec: metadata.ctime_nsec(),
        }
    }
}

/// storageへ渡すコピー元。読み取り専用で開いた1ファイルを表す。
///
/// - `handle_snapshot`は開いたハンドル自体の属性（fstat相当）。
/// - `entry_snapshot`はroot側のエントリを辿り直した属性（no-followのlstat相当）。
///   差し替え・移動・削除を検出するため、ハンドルとは独立に取得すること。
///
/// コピーは先頭へseekしてから行う。
pub trait CopySource: Read + Seek {
    fn handle_snapshot(&self) -> io::Result<SourceSnapshot>;
    fn entry_snapshot(&self) -> io::Result<SourceSnapshot>;
}

/// 絶対パスで開いたファイルの単純な実装。テストと統合前の暫定利用向け。
/// root配下のno-follow検査はタスク06の`OpenedSource`側の責務。
#[derive(Debug)]
pub struct FileSource {
    file: File,
    path: PathBuf,
}

impl FileSource {
    pub fn open(path: impl Into<PathBuf>) -> io::Result<Self> {
        let path = path.into();
        let file = File::open(&path)?;
        if !file.metadata()?.is_file() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "通常ファイルではありません",
            ));
        }
        Ok(Self { file, path })
    }
}

impl Read for FileSource {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.file.read(buf)
    }
}

impl Seek for FileSource {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.file.seek(pos)
    }
}

impl CopySource for FileSource {
    fn handle_snapshot(&self) -> io::Result<SourceSnapshot> {
        Ok(SourceSnapshot::from_metadata(&self.file.metadata()?))
    }

    fn entry_snapshot(&self) -> io::Result<SourceSnapshot> {
        Ok(SourceSnapshot::from_metadata(&std::fs::symlink_metadata(
            &self.path,
        )?))
    }
}
