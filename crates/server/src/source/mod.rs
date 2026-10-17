//! 設定rootのdescriptorを起点とした読み取り専用の走査とopen。
//!
//! 各階層を`openat`+`O_NOFOLLOW`で開き、シンボリックリンク・特殊ファイル・`..`・
//! 絶対パスを拒否する。パスはバイト列のまま扱い、非UTF-8名を損失変換しない。
use rustix::fs::{AtFlags, Dir, FileType, Mode, OFlags, open, openat, statat};
use rustix::io::Errno;
use rustune_contracts::error::ErrorCode;
use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::fs::File;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::path::{Component, Path, PathBuf};

/// 分類済みの失敗のみ。OSの生エラーや絶対パスは保持しない。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceError {
    /// `..`・絶対・空・深すぎる相対パス。
    InvalidPath,
    /// 設定root自体が存在しない、シンボリックリンク、またはディレクトリでない。
    RootUnavailable,
    PermissionDenied,
    /// 列挙からopenまでの間にentryが消えた。
    NotFound,
    /// 通常ファイルの位置にあるシンボリックリンク・FIFO・socket・デバイス・ディレクトリ。
    NotRegular,
    /// 属性またはファイル識別情報が観測済みfingerprintと異なる。
    Changed,
    LimitExceeded,
    Io,
}
impl SourceError {
    fn from_errno(errno: Errno) -> Self {
        match errno {
            Errno::ACCESS | Errno::PERM => Self::PermissionDenied,
            Errno::NOENT => Self::NotFound,
            // O_NOFOLLOWはシンボリックリンクをELOOP（一部BSDではEMLINK）、
            // O_DIRECTORYは非ディレクトリ・リンクをENOTDIRとして返す。
            Errno::LOOP | Errno::MLINK | Errno::NOTDIR => Self::NotRegular,
            _ => Self::Io,
        }
    }
    /// 取り込み項目・ジョブに出す契約code。OSのエラー文字列は含めない。
    pub fn error_code(self) -> ErrorCode {
        match self {
            Self::InvalidPath => ErrorCode::InvalidInput,
            Self::RootUnavailable => ErrorCode::InvalidSourceRoot,
            Self::PermissionDenied => ErrorCode::SourcePermissionDenied,
            // 列挙後に差し替え・削除された候補は次回の走査で再評価する。
            Self::NotFound | Self::NotRegular | Self::Changed => ErrorCode::FileUnstable,
            Self::LimitExceeded | Self::Io => ErrorCode::IoFailure,
        }
    }
    pub fn retryable(self) -> bool {
        !matches!(self, Self::InvalidPath)
    }
}

/// 変更検出用の属性。device・inodeでファイル同一性、size・mtime・ctimeで内容変化を見る。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fingerprint {
    pub device: u64,
    pub inode: u64,
    pub size: u64,
    pub modified_seconds: i64,
    pub modified_nanos: i64,
    pub changed_seconds: i64,
    pub changed_nanos: i64,
}
/// 開いたハンドルからfingerprintを取得する（パスを辿り直さない）。
pub fn fingerprint(file: &File) -> Result<Fingerprint, SourceError> {
    let metadata = file.metadata().map_err(|_| SourceError::Io)?;
    Ok(Fingerprint {
        device: metadata.dev(),
        inode: metadata.ino(),
        size: metadata.len(),
        modified_seconds: metadata.mtime(),
        modified_nanos: metadata.mtime_nsec(),
        changed_seconds: metadata.ctime(),
        changed_nanos: metadata.ctime_nsec(),
    })
}

/// 読み取り専用ハンドルとopen時の属性。絶対パスは持たず、他モジュールに自由なopenをさせない。
pub struct OpenedSource {
    file: File,
    initial: Fingerprint,
}
impl OpenedSource {
    pub fn reader(&mut self) -> &mut File {
        &mut self.file
    }
    pub fn initial_fingerprint(&self) -> Fingerprint {
        self.initial
    }
    /// ハンドルの現在の属性がopen時と一致するかを確認する。
    pub fn verify_unchanged(&self) -> Result<(), SourceError> {
        if fingerprint(&self.file)? == self.initial {
            Ok(())
        } else {
            Err(SourceError::Changed)
        }
    }
}

/// 列挙された候補。`relative_path`はroot相対のバイト列パス。
#[derive(Clone, Debug)]
pub struct Candidate {
    pub relative_path: PathBuf,
    pub fingerprint: Fingerprint,
}
/// 走査結果の集計。候補・警告件数と対象外ファイルの拡張子別件数。
#[derive(Clone, Debug, Default)]
pub struct ScanReport {
    pub candidates: u64,
    pub warnings: u64,
    pub ignored_extensions: BTreeMap<String, u64>,
    pub complete: bool,
}
/// 設定rootのディレクトリdescriptor。以降のopenはすべてこれを起点にする。
pub struct SourceRoot {
    directory: File,
}

fn directory_flags() -> OFlags {
    OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC
}
/// `statat`の結果と開いたハンドルが同じファイルか。st_devの型はOSで異なるため、
/// std::fs::Metadataと同じ`as u64`変換で比較する。
#[allow(clippy::unnecessary_cast)]
fn same_file(opened: &Fingerprint, stat: &rustix::fs::Stat) -> bool {
    opened.inode == stat.st_ino as u64 && opened.device == stat.st_dev as u64
}
fn root_error(errno: Errno) -> SourceError {
    match SourceError::from_errno(errno) {
        SourceError::PermissionDenied => SourceError::PermissionDenied,
        _ => SourceError::RootUnavailable,
    }
}
fn checked_components(path: &Path) -> Result<Vec<&OsStr>, SourceError> {
    let components: Vec<_> = path
        .components()
        .map(|component| match component {
            Component::Normal(name) => Ok(name),
            _ => Err(SourceError::InvalidPath),
        })
        .collect::<Result<_, _>>()?;
    if components.is_empty() || components.len() > 128 {
        return Err(SourceError::InvalidPath);
    }
    Ok(components)
}

impl SourceRoot {
    /// 絶対パスの設定rootを開く。root自身がシンボリックリンクなら拒否する。
    pub fn open(path: &Path) -> Result<Self, SourceError> {
        if !path.is_absolute() {
            return Err(SourceError::InvalidPath);
        }
        // macOSの/var -> /private/varのような親の別名は設定rootの外側として許す。
        // root自身と配下の各階層はno-followで開く。
        let directory =
            File::from(open(path, directory_flags(), Mode::empty()).map_err(root_error)?);
        Ok(Self { directory })
    }

    /// root相対パスを各階層no-followで辿り、通常ファイルだけを読み取り専用で開く。
    pub fn open_readonly(&self, relative: &Path) -> Result<OpenedSource, SourceError> {
        let components = checked_components(relative)?;
        let mut parent = self.directory.try_clone().map_err(|_| SourceError::Io)?;
        for name in &components[..components.len() - 1] {
            parent = File::from(
                openat(&parent, *name, directory_flags(), Mode::empty())
                    .map_err(SourceError::from_errno)?,
            );
        }
        let fd = openat(
            &parent,
            components[components.len() - 1],
            OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .map_err(SourceError::from_errno)?;
        let file = File::from(fd);
        if !file.metadata().map_err(|_| SourceError::Io)?.is_file() {
            return Err(SourceError::NotRegular);
        }
        let initial = fingerprint(&file)?;
        Ok(OpenedSource { file, initial })
    }

    /// 候補を開き、列挙時のfingerprintから変化していれば`Changed`を返す。
    pub fn open_candidate(&self, candidate: &Candidate) -> Result<OpenedSource, SourceError> {
        let source = self.open_readonly(&candidate.relative_path)?;
        if source.initial != candidate.fingerprint {
            return Err(SourceError::Changed);
        }
        Ok(source)
    }

    /// root側のentryが今も開いたハンドルと同じファイル・同じ属性かを確認する。
    /// コピー前後など、検査後の差し替えを検出するために使う。
    pub fn verify_entry(&self, relative: &Path, opened: &OpenedSource) -> Result<(), SourceError> {
        opened.verify_unchanged()?;
        let current = self.open_readonly(relative)?;
        if current.initial == opened.initial {
            Ok(())
        } else {
            Err(SourceError::Changed)
        }
    }

    /// 候補ごとにcallbackを1回呼び、上限のない候補一覧を保持しない。
    /// 警告が1件でもあれば`complete=false`とし、呼び出し側は全件確認済みと表示しない。
    /// root自体を読めない場合だけ`Err`を返す（ジョブ失敗）。
    pub fn enumerate(&self, mut visit: impl FnMut(Candidate)) -> Result<ScanReport, SourceError> {
        let mut report = ScanReport {
            complete: true,
            ..ScanReport::default()
        };
        let mut budget = 1_000_000_u64;
        self.walk(
            &self.directory,
            Path::new(""),
            0,
            &mut budget,
            &mut report,
            &mut visit,
        )?;
        report.complete = report.warnings == 0;
        Ok(report)
    }

    fn walk(
        &self,
        directory: &File,
        relative: &Path,
        depth: usize,
        budget: &mut u64,
        report: &mut ScanReport,
        visit: &mut impl FnMut(Candidate),
    ) -> Result<(), SourceError> {
        if depth > 128 {
            return Err(SourceError::LimitExceeded);
        }
        let mut entries = Dir::read_from(directory).map_err(SourceError::from_errno)?;
        while let Some(entry) = entries.read() {
            *budget = budget.checked_sub(1).ok_or(SourceError::LimitExceeded)?;
            let Ok(entry) = entry else {
                report.warnings += 1;
                continue;
            };
            let name = OsStr::from_bytes(entry.file_name().to_bytes());
            if name == "." || name == ".." {
                continue;
            }
            let Ok(stat) = statat(directory, name, AtFlags::SYMLINK_NOFOLLOW) else {
                report.warnings += 1;
                continue;
            };
            let path = relative.join(name);
            match FileType::from_raw_mode(stat.st_mode) {
                FileType::Directory => {
                    let Ok(fd) = openat(directory, name, directory_flags(), Mode::empty()) else {
                        report.warnings += 1;
                        continue;
                    };
                    let child = File::from(fd);
                    let Ok(attrs) = fingerprint(&child) else {
                        report.warnings += 1;
                        continue;
                    };
                    if !same_file(&attrs, &stat) {
                        report.warnings += 1;
                        continue;
                    }
                    if let Err(error) = self.walk(&child, &path, depth + 1, budget, report, visit) {
                        if error == SourceError::LimitExceeded {
                            return Err(error);
                        }
                        report.warnings += 1;
                    }
                }
                FileType::RegularFile => {
                    let extension = path.extension().map(OsStr::as_bytes).unwrap_or_default();
                    if extension.eq_ignore_ascii_case(b"m4a")
                        || extension.eq_ignore_ascii_case(b"m4p")
                    {
                        match self.open_readonly(&path) {
                            Ok(source) if same_file(&source.initial, &stat) => {
                                report.candidates += 1;
                                visit(Candidate {
                                    relative_path: path,
                                    fingerprint: source.initial,
                                });
                            }
                            _ => report.warnings += 1,
                        }
                    } else {
                        // 対象外ファイルは拡張子別件数だけを残す。異常な名前と
                        // 拡張子の種類数にも上限を設ける。
                        let label = if extension.is_empty() {
                            "none".into()
                        } else if extension.len() <= 32
                            && extension.iter().all(u8::is_ascii_alphanumeric)
                        {
                            String::from_utf8(extension.to_ascii_lowercase())
                                .expect("ASCII extension")
                        } else {
                            "other".into()
                        };
                        let label = if report.ignored_extensions.len() < 128
                            || report.ignored_extensions.contains_key(&label)
                        {
                            label
                        } else {
                            "other".into()
                        };
                        *report.ignored_extensions.entry(label).or_default() += 1;
                    }
                }
                _ => report.warnings += 1,
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
