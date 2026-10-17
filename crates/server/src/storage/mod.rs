//! 管理下の保存先（staging・media・artwork・quarantine）のファイル管理。
//!
//! - 保存先のファイル名はUUIDだけから組み立て、曲名などの利用者入力をパスに使わない。
//! - 呼び出し側は任意のパスを渡せず、[`ManagedFile`]で対象を指定する。管理範囲外を
//!   削除・移動するAPIは提供しない。
//! - 完成音源は`media/`にしか存在せず、`media/`へは検証済みの[`StagedFile`]を
//!   [`Storage::promote`]でrenameした場合だけ入る。コピー途中・失敗したファイルは
//!   `staging/*.part`に留まり、完成音源として扱われない。
//! - DBトランザクションとジョブ管理は扱わない（タスク05・09・10）。

mod io;
mod source;

pub use io::{OsIo, StorageIo};
pub use source::{CopySource, FileSource, SourceSnapshot};

use std::fmt;
use std::fs::{self, DirBuilder, File, OpenOptions};
use std::io::{ErrorKind, Read, SeekFrom};
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use rustune_contracts::error::ErrorCode;
use rustune_domain::track::TrackId;
use sha2::{Digest, Sha256};
use uuid::Uuid;

const MIB: u64 = 1024 * 1024;
const GIB: u64 = 1024 * MIB;

const STAGING_DIR: &str = "staging";
const MEDIA_DIR: &str = "media";
const ARTWORK_DIR: &str = "artwork";
const QUARANTINE_DIR: &str = "quarantine";

/// コピーの資源制限。既定値はMODULE_DESIGN §4の`max_file`・`reserve_free`。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CopyLimits {
    /// 実際に読み出したバイト数にも適用する1ファイルの上限。
    pub max_file_bytes: u64,
    /// コピー後も保存先に残す空き容量の下限。
    pub reserve_free_bytes: u64,
    /// コピー中に空き容量を確認する間隔（バイト）。
    pub space_check_interval: u64,
    /// 固定サイズの読み書きbuffer。ファイル全体をメモリへ載せない。
    pub buffer_size: usize,
}

impl Default for CopyLimits {
    fn default() -> Self {
        Self {
            max_file_bytes: 200 * MIB,
            reserve_free_bytes: GIB,
            space_check_interval: 16 * MIB,
            buffer_size: 256 * 1024,
        }
    }
}

/// 管理下ファイルのSHA-256。DBのfile_digestと復旧manifestに使う。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FileDigest([u8; 32]);

impl FileDigest {
    pub fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
    pub fn bytes(&self) -> &[u8; 32] {
        &self.0
    }
    pub fn to_hex(&self) -> String {
        self.0.iter().map(|b| format!("{b:02x}")).collect()
    }
    /// 小文字16進64文字だけを受け付ける。
    pub fn from_hex(hex: &str) -> Option<Self> {
        if hex.len() != 64 || !hex.bytes().all(|c| matches!(c, b'0'..=b'9' | b'a'..=b'f')) {
            return None;
        }
        let mut out = [0u8; 32];
        for (i, slot) in out.iter_mut().enumerate() {
            *slot = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).ok()?;
        }
        Some(Self(out))
    }
}

/// ジャケット画像の表示用・一覧用の区別。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ArtworkVariant {
    /// 最大512px。
    Display,
    /// 最大128px。
    Thumbnail,
}

/// 保存する画像形式。再エンコード時は常にJPEG。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ArtworkFormat {
    Jpeg,
    Png,
}

impl ArtworkFormat {
    fn extension(self) -> &'static str {
        match self {
            Self::Jpeg => "jpg",
            Self::Png => "png",
        }
    }
}

/// 管理下ファイル1件の指定。パスはこの値からのみ組み立てる。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ManagedFile {
    /// `staging/<uuid>.part`
    Staging(Uuid),
    /// `media/<track uuid>.m4a`
    Media(TrackId),
    /// `artwork/<uuid>.jpg`、一覧用は`artwork/<uuid>.thumb.jpg`（PNG保持時は`.png`）。
    Artwork {
        id: Uuid,
        variant: ArtworkVariant,
        format: ArtworkFormat,
    },
}

/// [`Storage::list_managed`]で列挙する種類。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ManagedKind {
    Staging,
    Media,
    Artwork,
}

impl ManagedFile {
    /// 保存先rootからの相対パス（DBのmedia_path・artworks.pathに保存する値）。
    pub fn relative_path(&self) -> String {
        match *self {
            Self::Staging(id) => format!("{STAGING_DIR}/{}.part", id.hyphenated()),
            Self::Media(track) => format!("{MEDIA_DIR}/{}.m4a", track.as_uuid().hyphenated()),
            Self::Artwork {
                id,
                variant,
                format,
            } => {
                let infix = match variant {
                    ArtworkVariant::Display => "",
                    ArtworkVariant::Thumbnail => ".thumb",
                };
                format!(
                    "{ARTWORK_DIR}/{}{infix}.{}",
                    id.hyphenated(),
                    format.extension()
                )
            }
        }
    }

    fn dir(&self) -> &'static str {
        match self {
            Self::Staging(_) => STAGING_DIR,
            Self::Media(_) => MEDIA_DIR,
            Self::Artwork { .. } => ARTWORK_DIR,
        }
    }

    /// 管理下の正規の名前だけを解釈する。大文字UUIDや余分な接尾辞は対象外。
    fn parse(kind: ManagedKind, name: &str) -> Option<Self> {
        match kind {
            ManagedKind::Staging => canonical_uuid(name.strip_suffix(".part")?).map(Self::Staging),
            ManagedKind::Media => canonical_uuid(name.strip_suffix(".m4a")?)
                .and_then(|id| TrackId::new(id).ok())
                .map(Self::Media),
            ManagedKind::Artwork => {
                let (stem, format) = if let Some(stem) = name.strip_suffix(".jpg") {
                    (stem, ArtworkFormat::Jpeg)
                } else {
                    (name.strip_suffix(".png")?, ArtworkFormat::Png)
                };
                let (stem, variant) = match stem.strip_suffix(".thumb") {
                    Some(stem) => (stem, ArtworkVariant::Thumbnail),
                    None => (stem, ArtworkVariant::Display),
                };
                canonical_uuid(stem).map(|id| Self::Artwork {
                    id,
                    variant,
                    format,
                })
            }
        }
    }
}

fn canonical_uuid(text: &str) -> Option<Uuid> {
    let id = Uuid::try_parse(text).ok()?;
    (!id.is_nil() && id.hyphenated().to_string() == text).then_some(id)
}

/// storageの失敗。`reason`は取り込み項目の理由code。
#[derive(Debug)]
pub enum StorageError {
    /// コピー前後で元ファイルの同一性・サイズ・更新時刻が変化した。
    FileUnstable,
    /// 予定サイズまたは実際の読み出し量が上限を超えた。
    FileTooLarge {
        limit: u64,
    },
    /// 空き容量が「残りコピー量 + 下限」を下回った。
    StorageFull {
        required: u64,
        available: u64,
    },
    /// 協調キャンセルされた。
    Cancelled,
    /// 移動先が既に存在する。上書きはしない。
    AlreadyExists,
    /// 保存済みファイルのサイズ・digestが期待値と一致しない。
    IntegrityMismatch,
    /// 保存先ディレクトリの構成が不正（シンボリックリンク・通常ファイル等）。
    InvalidLayout(&'static str),
    /// この操作で扱えない管理対象が指定された。
    InvalidTarget,
    Io(std::io::Error),
}

impl StorageError {
    /// 取り込み項目・APIの理由code。`file_too_large`はcontractsに未定義（PR参照）。
    pub fn reason(&self) -> &'static str {
        match self {
            Self::FileUnstable => ErrorCode::FileUnstable.as_str(),
            Self::FileTooLarge { .. } => "file_too_large",
            Self::StorageFull { .. } => ErrorCode::StorageFull.as_str(),
            Self::Cancelled => ErrorCode::Interrupted.as_str(),
            Self::AlreadyExists
            | Self::IntegrityMismatch
            | Self::InvalidLayout(_)
            | Self::InvalidTarget
            | Self::Io(_) => ErrorCode::IoFailure.as_str(),
        }
    }

    /// 同じ入力のまま後で再試行して成功し得るか。
    pub fn retryable(&self) -> bool {
        !matches!(
            self,
            Self::FileTooLarge { .. } | Self::InvalidLayout(_) | Self::InvalidTarget
        )
    }
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FileUnstable => f.write_str("コピー中に元ファイルが変化しました"),
            Self::FileTooLarge { limit } => write!(f, "ファイルが上限{limit}バイトを超えています"),
            Self::StorageFull {
                required,
                available,
            } => write!(
                f,
                "空き容量が不足しています（必要{required}バイト、空き{available}バイト）"
            ),
            Self::Cancelled => f.write_str("コピーがキャンセルされました"),
            Self::AlreadyExists => f.write_str("保存先に同名のファイルがあります"),
            Self::IntegrityMismatch => f.write_str("保存ファイルの内容が期待値と一致しません"),
            Self::InvalidLayout(what) => write!(f, "保存先の構成が不正です: {what}"),
            Self::InvalidTarget => f.write_str("この操作で扱えない管理対象です"),
            Self::Io(error) => write!(f, "ファイル操作に失敗しました: {error}"),
        }
    }
}

impl std::error::Error for StorageError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            _ => None,
        }
    }
}

impl From<std::io::Error> for StorageError {
    fn from(error: std::io::Error) -> Self {
        if error.kind() == ErrorKind::AlreadyExists {
            Self::AlreadyExists
        } else {
            Self::Io(error)
        }
    }
}

/// 書き込み中のstagingファイル。コピーに使われずにdropされた場合は削除する。
#[derive(Debug)]
pub struct Staging {
    id: Uuid,
    track_id: TrackId,
    path: PathBuf,
    file: Option<File>,
}

impl Staging {
    pub fn id(&self) -> Uuid {
        self.id
    }
    pub fn track_id(&self) -> TrackId {
        self.track_id
    }
    pub fn managed(&self) -> ManagedFile {
        ManagedFile::Staging(self.id)
    }
}

impl Drop for Staging {
    fn drop(&mut self) {
        if self.file.take().is_some() {
            let _ = fs::remove_file(&self.path);
        }
    }
}

/// コピーと検査を完了し、syncしたstagingファイル。まだ完成音源ではない。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StagedFile {
    staging_id: Uuid,
    track_id: TrackId,
    size: u64,
    digest: FileDigest,
}

impl StagedFile {
    pub fn staging_id(&self) -> Uuid {
        self.staging_id
    }
    pub fn track_id(&self) -> TrackId {
        self.track_id
    }
    pub fn size(&self) -> u64 {
        self.size
    }
    pub fn digest(&self) -> FileDigest {
        self.digest
    }
    pub fn staging(&self) -> ManagedFile {
        ManagedFile::Staging(self.staging_id)
    }
    /// promotion後の最終保存先。
    pub fn target(&self) -> ManagedFile {
        ManagedFile::Media(self.track_id)
    }
}

/// `media/`へ確定した音源。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MediaFile {
    pub track_id: TrackId,
    pub relative_path: String,
    pub size: u64,
    pub digest: FileDigest,
}

/// quarantineへ移した管理下ファイル。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuarantinedFile {
    pub original: ManagedFile,
    pub relative_path: String,
}

/// 専用データディレクトリ配下の管理下ファイル操作。
#[derive(Clone)]
pub struct Storage {
    root: PathBuf,
    io: Arc<dyn StorageIo>,
}

impl fmt::Debug for Storage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Storage").field("root", &self.root).finish()
    }
}

impl Storage {
    /// 既存の保存先rootを開き、管理用サブディレクトリを0700で用意する。
    pub fn open(root: impl Into<PathBuf>) -> Result<Self, StorageError> {
        Self::with_io(root, Arc::new(OsIo))
    }

    /// I/O境界を差し替えて開く（障害注入テスト用）。
    pub fn with_io(root: impl Into<PathBuf>, io: Arc<dyn StorageIo>) -> Result<Self, StorageError> {
        let root = root.into();
        if !fs::metadata(&root)?.is_dir() {
            return Err(StorageError::InvalidLayout(
                "保存先rootがディレクトリではありません",
            ));
        }
        let storage = Self { root, io };
        let mut created = false;
        for dir in [STAGING_DIR, MEDIA_DIR, ARTWORK_DIR, QUARANTINE_DIR] {
            let path = storage.root.join(dir);
            match fs::symlink_metadata(&path) {
                Ok(meta) if meta.is_dir() => {}
                Ok(_) => {
                    return Err(StorageError::InvalidLayout(
                        "管理用サブディレクトリが実ディレクトリではありません",
                    ));
                }
                Err(error) if error.kind() == ErrorKind::NotFound => {
                    DirBuilder::new().mode(0o700).create(&path)?;
                    created = true;
                }
                Err(error) => return Err(error.into()),
            }
        }
        if created {
            storage.io.sync_dir(&storage.root)?;
        }
        Ok(storage)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// 管理下ファイルの絶対パス。配信など読み取り用途に限る。
    pub fn path_of(&self, file: ManagedFile) -> PathBuf {
        self.root.join(file.relative_path())
    }

    fn dir(&self, name: &str) -> PathBuf {
        self.root.join(name)
    }

    /// 新しいUUIDで`staging/<uuid>.part`を排他作成する。最終名はtrack IDから決まる。
    pub fn allocate_staging(&self, track_id: TrackId) -> Result<Staging, StorageError> {
        let id = Uuid::new_v4();
        let path = self.path_of(ManagedFile::Staging(id));
        let file = create_new(&path)?;
        Ok(Staging {
            id,
            track_id,
            path,
            file: Some(file),
        })
    }

    /// 元ファイルを固定bufferでstagingへコピーし、digestを計算してsyncする。
    ///
    /// 開始前に「予定サイズ + 空き容量下限」を確認し、コピー中も一定バイトごとに
    /// 「残り + 下限」を確認する。元ハンドルとroot側エントリの属性をコピー前後に
    /// 照合し、変化があればstagingを破棄して[`StorageError::FileUnstable`]を返す。
    /// 失敗時はstagingファイルを削除する（削除にも失敗した場合は復旧処理が回収する）。
    pub fn copy_checked<S: CopySource + ?Sized>(
        &self,
        source: &mut S,
        mut staging: Staging,
        limits: &CopyLimits,
        cancel: &AtomicBool,
    ) -> Result<StagedFile, StorageError> {
        match self.copy_into(source, &mut staging, limits, cancel) {
            Ok((size, digest)) => {
                staging.file.take();
                Ok(StagedFile {
                    staging_id: staging.id,
                    track_id: staging.track_id,
                    size,
                    digest,
                })
            }
            Err(error) => {
                staging.file.take();
                let _ = self.io.remove_file(&staging.path);
                Err(error)
            }
        }
    }

    fn copy_into<S: CopySource + ?Sized>(
        &self,
        source: &mut S,
        staging: &mut Staging,
        limits: &CopyLimits,
        cancel: &AtomicBool,
    ) -> Result<(u64, FileDigest), StorageError> {
        let before = source.handle_snapshot()?;
        let entry = entry_snapshot(source)?;
        if before != entry {
            return Err(StorageError::FileUnstable);
        }
        if before.size > limits.max_file_bytes {
            return Err(StorageError::FileTooLarge {
                limit: limits.max_file_bytes,
            });
        }
        self.ensure_space(before.size, limits)?;

        source.seek(SeekFrom::Start(0))?;
        let file = staging.file.as_mut().ok_or(StorageError::InvalidTarget)?;
        let mut buffer = vec![0u8; limits.buffer_size.max(1)];
        let mut hasher = Sha256::new();
        let mut total: u64 = 0;
        let mut since_check: u64 = 0;
        loop {
            if cancel.load(Ordering::Relaxed) {
                return Err(StorageError::Cancelled);
            }
            let read = match source.read(&mut buffer) {
                Ok(0) => break,
                Ok(read) => read,
                Err(error) if error.kind() == ErrorKind::Interrupted => continue,
                Err(error) => return Err(error.into()),
            };
            total = total.saturating_add(read as u64);
            if total > limits.max_file_bytes {
                return Err(StorageError::FileTooLarge {
                    limit: limits.max_file_bytes,
                });
            }
            if total > before.size {
                return Err(StorageError::FileUnstable);
            }
            self.io.write_all(file, &buffer[..read])?;
            hasher.update(&buffer[..read]);
            since_check = since_check.saturating_add(read as u64);
            if since_check >= limits.space_check_interval {
                since_check = 0;
                self.ensure_space(before.size - total, limits)?;
            }
        }
        if total != before.size
            || source.handle_snapshot()? != before
            || entry_snapshot(source)? != before
        {
            return Err(StorageError::FileUnstable);
        }
        self.io.sync_file(file)?;
        self.io.sync_dir(&self.dir(STAGING_DIR))?;
        Ok((total, FileDigest(hasher.finalize().into())))
    }

    fn ensure_space(&self, remaining: u64, limits: &CopyLimits) -> Result<(), StorageError> {
        let available = self.io.available_space(&self.dir(STAGING_DIR))?;
        let required = remaining.saturating_add(limits.reserve_free_bytes);
        if available < required {
            return Err(StorageError::StorageFull {
                required,
                available,
            });
        }
        Ok(())
    }

    /// 復旧manifestの値でstagingファイルを再検証する（サイズ・digestの全読み）。
    pub fn verify_staged(
        &self,
        staging_id: Uuid,
        track_id: TrackId,
        expected_size: u64,
        expected_digest: FileDigest,
    ) -> Result<StagedFile, StorageError> {
        let managed = ManagedFile::Staging(staging_id);
        let (size, digest) = self.digest_managed(managed)?;
        if size != expected_size || digest != expected_digest {
            return Err(StorageError::IntegrityMismatch);
        }
        Ok(StagedFile {
            staging_id,
            track_id,
            size,
            digest,
        })
    }

    /// 管理下ファイルのサイズとSHA-256を固定bufferで計算する。
    pub fn digest_managed(&self, file: ManagedFile) -> Result<(u64, FileDigest), StorageError> {
        let path = self.path_of(file);
        if !fs::symlink_metadata(&path)?.is_file() {
            return Err(StorageError::InvalidTarget);
        }
        let mut handle = File::open(&path)?;
        let mut buffer = vec![0u8; CopyLimits::default().buffer_size];
        let mut hasher = Sha256::new();
        let mut total = 0u64;
        loop {
            let read = match handle.read(&mut buffer) {
                Ok(0) => break,
                Ok(read) => read,
                Err(error) if error.kind() == ErrorKind::Interrupted => continue,
                Err(error) => return Err(error.into()),
            };
            hasher.update(&buffer[..read]);
            total += read as u64;
        }
        Ok((total, FileDigest(hasher.finalize().into())))
    }

    /// stagingファイルをsyncし、同じファイルシステム上で`media/<track>.m4a`へ
    /// 置換なしでrenameし、移動先・移動元の親ディレクトリをsyncする。
    ///
    /// rename前に失敗した場合はstagingに残り、`media/`には何も現れない。
    /// rename後のディレクトリsyncで失敗した場合は最終名に存在するがエラーを返す
    /// （復旧の「staging行、最終ファイルあり」で再確認する）。
    pub fn promote(&self, staged: &StagedFile) -> Result<MediaFile, StorageError> {
        let from = self.path_of(staged.staging());
        let target = staged.target();
        let to = self.path_of(target);
        let meta = fs::symlink_metadata(&from)?;
        if !meta.is_file() || meta.len() != staged.size {
            return Err(StorageError::IntegrityMismatch);
        }
        let handle = File::open(&from)?;
        self.io.sync_file(&handle)?;
        drop(handle);
        self.io.rename_noreplace(&from, &to)?;
        self.io.sync_dir(&self.dir(MEDIA_DIR))?;
        self.io.sync_dir(&self.dir(STAGING_DIR))?;
        Ok(MediaFile {
            track_id: staged.track_id,
            relative_path: target.relative_path(),
            size: staged.size,
            digest: staged.digest,
        })
    }

    /// 小さなバイト列（ジャケット画像）をstaging経由で管理下へ書き込み、確定する。
    /// 対象はartworkのみ。既存ファイルは上書きしない。
    pub fn install_bytes(&self, target: ManagedFile, bytes: &[u8]) -> Result<(), StorageError> {
        if !matches!(target, ManagedFile::Artwork { .. }) {
            return Err(StorageError::InvalidTarget);
        }
        let temp = self.path_of(ManagedFile::Staging(Uuid::new_v4()));
        let result = (|| {
            let mut file = create_new(&temp)?;
            self.io.write_all(&mut file, bytes)?;
            self.io.sync_file(&file)?;
            drop(file);
            self.io.rename_noreplace(&temp, &self.path_of(target))
        })();
        if let Err(error) = result {
            let _ = self.io.remove_file(&temp);
            return Err(error.into());
        }
        self.io.sync_dir(&self.dir(target.dir()))?;
        self.io.sync_dir(&self.dir(STAGING_DIR))?;
        Ok(())
    }

    /// 管理下ファイルを削除する。既にない場合は成功。その他のI/O失敗はそのまま返し、
    /// 呼び出し側は状態（deleting等）を保持して再試行する。
    pub fn remove_managed(&self, file: ManagedFile) -> Result<(), StorageError> {
        let path = self.path_of(file);
        match fs::symlink_metadata(&path) {
            Ok(meta) if meta.is_dir() => return Err(StorageError::InvalidTarget),
            Ok(_) => {}
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(error.into()),
        }
        match self.io.remove_file(&path) {
            Ok(()) => {}
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(error.into()),
        }
        self.io.sync_dir(&self.dir(file.dir()))?;
        Ok(())
    }

    /// stagingを破棄する（[`Self::remove_managed`]の別名）。
    pub fn discard_staged(&self, staged: &StagedFile) -> Result<(), StorageError> {
        self.remove_managed(staged.staging())
    }

    /// DBにない管理下ファイルを直ちに消さず、`quarantine/<新uuid>-<元の名前>`へ移す。
    pub fn quarantine(&self, file: ManagedFile) -> Result<QuarantinedFile, StorageError> {
        let from = self.path_of(file);
        if !fs::symlink_metadata(&from)?.is_file() {
            return Err(StorageError::InvalidTarget);
        }
        let original = file.relative_path();
        let name = original.rsplit('/').next().unwrap_or_default();
        let relative_path = format!("{QUARANTINE_DIR}/{}-{name}", Uuid::new_v4().hyphenated());
        self.io
            .rename_noreplace(&from, &self.root.join(&relative_path))?;
        self.io.sync_dir(&self.dir(QUARANTINE_DIR))?;
        self.io.sync_dir(&self.dir(file.dir()))?;
        Ok(QuarantinedFile {
            original: file,
            relative_path,
        })
    }

    /// 管理用ディレクトリ内の正規の名前を持つ通常ファイルを列挙する（復旧用）。
    /// それ以外の名前・ディレクトリ・リンクは含めない。
    pub fn list_managed(&self, kind: ManagedKind) -> Result<Vec<ManagedFile>, StorageError> {
        let dir = match kind {
            ManagedKind::Staging => STAGING_DIR,
            ManagedKind::Media => MEDIA_DIR,
            ManagedKind::Artwork => ARTWORK_DIR,
        };
        let mut found = Vec::new();
        for entry in fs::read_dir(self.dir(dir))? {
            let entry = entry?;
            let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
                continue;
            };
            if !entry.file_type()?.is_file() {
                continue;
            }
            if let Some(file) = ManagedFile::parse(kind, &name) {
                found.push(file);
            }
        }
        found.sort_by_key(|file| file.relative_path());
        Ok(found)
    }
}

fn create_new(path: &Path) -> std::io::Result<File> {
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
}

fn entry_snapshot<S: CopySource + ?Sized>(source: &S) -> Result<SourceSnapshot, StorageError> {
    source.entry_snapshot().map_err(|error| match error.kind() {
        ErrorKind::NotFound => StorageError::FileUnstable,
        _ => StorageError::Io(error),
    })
}
