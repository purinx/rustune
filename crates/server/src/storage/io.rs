//! 障害注入のために差し替え可能なファイルI/O境界。
//!
//! 製品コードは[`OsIo`]を使う。テストは書き込み・sync・rename・削除・空き容量の
//! 各操作を個別に失敗させる実装へ差し替える。

use std::fs::File;
use std::io::{self, Write};
use std::path::Path;

/// storageが行う副作用のある操作。各メソッドは1回のOS操作に対応する。
pub trait StorageIo: Send + Sync {
    /// `dir`を含むファイルシステムで、一般ユーザーが使える空き容量（バイト）。
    fn available_space(&self, dir: &Path) -> io::Result<u64>;
    /// `buf`全体を書き込む。
    fn write_all(&self, file: &mut File, buf: &[u8]) -> io::Result<()>;
    /// ファイル内容とメタデータを永続化する（macOSではF_FULLFSYNC）。
    fn sync_file(&self, file: &File) -> io::Result<()>;
    /// 置換しないrename。移動先が既にあれば`AlreadyExists`。
    fn rename_noreplace(&self, from: &Path, to: &Path) -> io::Result<()>;
    /// ディレクトリのエントリ変更を永続化する。
    fn sync_dir(&self, dir: &Path) -> io::Result<()>;
    /// ファイルを1つ削除する。
    fn remove_file(&self, path: &Path) -> io::Result<()>;
}

/// OSのファイルAPIをそのまま使う実装。
#[derive(Clone, Copy, Debug, Default)]
pub struct OsIo;

impl StorageIo for OsIo {
    fn available_space(&self, dir: &Path) -> io::Result<u64> {
        let stat = rustix::fs::statvfs(dir)?;
        Ok(stat.f_bavail.saturating_mul(stat.f_frsize))
    }

    fn write_all(&self, file: &mut File, buf: &[u8]) -> io::Result<()> {
        file.write_all(buf)
    }

    fn sync_file(&self, file: &File) -> io::Result<()> {
        file.sync_all()
    }

    fn rename_noreplace(&self, from: &Path, to: &Path) -> io::Result<()> {
        rustix::fs::renameat_with(
            rustix::fs::CWD,
            from,
            rustix::fs::CWD,
            to,
            rustix::fs::RenameFlags::NOREPLACE,
        )?;
        Ok(())
    }

    fn sync_dir(&self, dir: &Path) -> io::Result<()> {
        let handle = File::open(dir)?;
        rustix::fs::fsync(&handle)?;
        Ok(())
    }

    fn remove_file(&self, path: &Path) -> io::Result<()> {
        std::fs::remove_file(path)
    }
}
