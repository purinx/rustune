//! storageのコピー・確定・削除・quarantineと障害注入のテスト。合成データのみを使う。

mod support;

use std::fs;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use rustune_domain::track::TrackId;
use rustune_server::storage::{
    CopyLimits, CopySource, FileDigest, FileSource, ManagedFile, ManagedKind, SourceSnapshot,
    Storage, StorageError,
};
use sha2::{Digest, Sha256};
use support::FaultyIo;
use tempfile::TempDir;
use uuid::Uuid;

const SIZE: usize = 300_000;

struct Fixture {
    _dir: TempDir,
    data_root: PathBuf,
    source_dir: PathBuf,
}

fn fixture() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let data_root = dir.path().join("data");
    let source_dir = dir.path().join("source");
    fs::create_dir(&data_root).unwrap();
    fs::create_dir(&source_dir).unwrap();
    Fixture {
        _dir: dir,
        data_root,
        source_dir,
    }
}

fn synthetic(len: usize) -> Vec<u8> {
    (0..len).map(|i| (i * 31 % 251) as u8).collect()
}

fn write_source(dir: &Path, name: &str, bytes: &[u8]) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, bytes).unwrap();
    path
}

fn track() -> TrackId {
    TrackId::new(Uuid::new_v4()).unwrap()
}

fn small_limits() -> CopyLimits {
    CopyLimits {
        max_file_bytes: 1024 * 1024,
        reserve_free_bytes: 0,
        space_check_interval: 64 * 1024,
        buffer_size: 4096,
    }
}

fn not_cancelled() -> AtomicBool {
    AtomicBool::new(false)
}

fn names(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    names.sort();
    names
}

fn faulty(fx: &Fixture, io: FaultyIo) -> (Storage, Arc<FaultyIo>) {
    let io = Arc::new(io);
    (Storage::with_io(&fx.data_root, io.clone()).unwrap(), io)
}

#[test]
fn copy_and_promote_keep_playable_copy_after_source_moves() {
    let fx = fixture();
    let data = synthetic(SIZE);
    let source_path = write_source(&fx.source_dir, "曲.m4a", &data);
    let before = fs::metadata(&source_path).unwrap();
    let storage = Storage::open(&fx.data_root).unwrap();
    for dir in ["staging", "media", "artwork", "quarantine"] {
        let meta = fs::metadata(fx.data_root.join(dir)).unwrap();
        assert_eq!(meta.permissions().mode() & 0o777, 0o700, "{dir}");
    }

    let id = track();
    let staging = storage.allocate_staging(id).unwrap();
    let mut source = FileSource::open(&source_path).unwrap();
    let staged = storage
        .copy_checked(&mut source, staging, &small_limits(), &not_cancelled())
        .unwrap();
    drop(source);

    assert_eq!(staged.size(), SIZE as u64);
    assert_eq!(
        staged.digest().bytes(),
        &<[u8; 32]>::from(Sha256::digest(&data))
    );
    // 確定前は完成音源として存在しない。
    assert!(names(&fx.data_root.join("media")).is_empty());
    assert_eq!(
        storage.list_managed(ManagedKind::Staging).unwrap(),
        vec![staged.staging()]
    );
    let staging_mode = fs::metadata(storage.path_of(staged.staging()))
        .unwrap()
        .permissions()
        .mode();
    assert_eq!(staging_mode & 0o777, 0o600);

    let media = storage.promote(&staged).unwrap();
    assert_eq!(media.relative_path, format!("media/{}.m4a", id.as_uuid()));
    assert!(names(&fx.data_root.join("staging")).is_empty());

    // 元は内容・サイズ・mtimeとも変化しない。
    let after = fs::metadata(&source_path).unwrap();
    assert_eq!(fs::read(&source_path).unwrap(), data);
    assert_eq!(after.size(), before.size());
    assert_eq!(
        (after.mtime(), after.mtime_nsec()),
        (before.mtime(), before.mtime_nsec())
    );

    // 元を移動・削除しても管理下コピーを読める。
    let moved = fx.source_dir.join("moved.m4a");
    fs::rename(&source_path, &moved).unwrap();
    fs::remove_file(&moved).unwrap();
    let copy = fs::read(fx.data_root.join(&media.relative_path)).unwrap();
    assert_eq!(copy, data);
    assert_eq!(
        storage.digest_managed(ManagedFile::Media(id)).unwrap(),
        (SIZE as u64, staged.digest())
    );
}

#[test]
fn partial_write_failure_leaves_no_staging_or_media() {
    let fx = fixture();
    let source_path = write_source(&fx.source_dir, "a.m4a", &synthetic(SIZE));
    let io = FaultyIo::default();
    *io.fail_write_after.lock().unwrap() = Some(10_000);
    let (storage, _) = faulty(&fx, io);
    let staging = storage.allocate_staging(track()).unwrap();
    let error = storage
        .copy_checked(
            &mut FileSource::open(&source_path).unwrap(),
            staging,
            &small_limits(),
            &not_cancelled(),
        )
        .unwrap_err();
    assert!(matches!(error, StorageError::Io(_)), "{error:?}");
    assert_eq!(error.reason(), "io_failure");
    assert!(names(&fx.data_root.join("staging")).is_empty());
    assert!(names(&fx.data_root.join("media")).is_empty());
}

#[test]
fn partial_staging_survives_only_as_part_when_cleanup_also_fails() {
    let fx = fixture();
    let source_path = write_source(&fx.source_dir, "a.m4a", &synthetic(SIZE));
    let io = FaultyIo::default();
    *io.fail_write_after.lock().unwrap() = Some(10_000);
    io.fail_remove.store(true, Ordering::SeqCst);
    let (storage, _) = faulty(&fx, io);
    let staging = storage.allocate_staging(track()).unwrap();
    let staging_id = staging.id();
    storage
        .copy_checked(
            &mut FileSource::open(&source_path).unwrap(),
            staging,
            &small_limits(),
            &not_cancelled(),
        )
        .unwrap_err();
    // 回収できなかった部分コピーは.partのままで、media/には現れない。
    assert_eq!(
        storage.list_managed(ManagedKind::Staging).unwrap(),
        vec![ManagedFile::Staging(staging_id)]
    );
    assert!(storage.list_managed(ManagedKind::Media).unwrap().is_empty());
}

#[test]
fn sync_failure_discards_staging() {
    let fx = fixture();
    let source_path = write_source(&fx.source_dir, "a.m4a", &synthetic(SIZE));
    let io = FaultyIo::default();
    io.fail_sync_file.store(true, Ordering::SeqCst);
    let (storage, _) = faulty(&fx, io);
    let staging = storage.allocate_staging(track()).unwrap();
    let error = storage
        .copy_checked(
            &mut FileSource::open(&source_path).unwrap(),
            staging,
            &small_limits(),
            &not_cancelled(),
        )
        .unwrap_err();
    assert!(matches!(error, StorageError::Io(_)));
    assert!(names(&fx.data_root.join("staging")).is_empty());
}

fn staged_with(fx: &Fixture, storage: &Storage) -> rustune_server::storage::StagedFile {
    let source_path = write_source(&fx.source_dir, "a.m4a", &synthetic(SIZE));
    let staging = storage.allocate_staging(track()).unwrap();
    storage
        .copy_checked(
            &mut FileSource::open(&source_path).unwrap(),
            staging,
            &small_limits(),
            &not_cancelled(),
        )
        .unwrap()
}

#[test]
fn rename_failure_keeps_staging_and_can_be_retried_after_verification() {
    let fx = fixture();
    let (storage, io) = faulty(&fx, FaultyIo::default());
    let staged = staged_with(&fx, &storage);
    *io.renames_before_failure.lock().unwrap() = Some(0);
    storage.promote(&staged).unwrap_err();
    assert!(storage.list_managed(ManagedKind::Media).unwrap().is_empty());
    assert_eq!(
        storage.list_managed(ManagedKind::Staging).unwrap(),
        vec![staged.staging()]
    );

    // 再起動後を想定し、manifestの値で再検証してから確定する。
    let restarted = Storage::open(&fx.data_root).unwrap();
    let verified = restarted
        .verify_staged(
            staged.staging_id(),
            staged.track_id(),
            staged.size(),
            FileDigest::from_hex(&staged.digest().to_hex()).unwrap(),
        )
        .unwrap();
    restarted.promote(&verified).unwrap();
    assert_eq!(
        restarted.list_managed(ManagedKind::Media).unwrap(),
        vec![staged.target()]
    );
}

#[test]
fn directory_sync_failure_after_rename_is_reported_with_final_file_present() {
    let fx = fixture();
    let (storage, io) = faulty(&fx, FaultyIo::default());
    let staged = staged_with(&fx, &storage);
    io.fail_sync_dir.store(true, Ordering::SeqCst);
    storage.promote(&staged).unwrap_err();
    // rename後の停止相当：最終ファイルはあり、復旧はdigestで再確認できる。
    assert!(
        storage
            .list_managed(ManagedKind::Staging)
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        storage.digest_managed(staged.target()).unwrap(),
        (staged.size(), staged.digest())
    );
}

#[test]
fn corrupted_staging_is_rejected_by_verification_and_promotion() {
    let fx = fixture();
    let storage = Storage::open(&fx.data_root).unwrap();
    let staged = staged_with(&fx, &storage);
    let path = storage.path_of(staged.staging());
    let mut file = fs::OpenOptions::new().append(true).open(&path).unwrap();
    file.write_all(b"x").unwrap();
    drop(file);
    assert!(matches!(
        storage.verify_staged(
            staged.staging_id(),
            staged.track_id(),
            staged.size(),
            staged.digest()
        ),
        Err(StorageError::IntegrityMismatch)
    ));
    assert!(matches!(
        storage.promote(&staged),
        Err(StorageError::IntegrityMismatch)
    ));
    assert!(storage.list_managed(ManagedKind::Media).unwrap().is_empty());
}

#[test]
fn promote_never_replaces_existing_media() {
    let fx = fixture();
    let storage = Storage::open(&fx.data_root).unwrap();
    let staged = staged_with(&fx, &storage);
    let existing = storage.path_of(staged.target());
    fs::write(&existing, b"existing").unwrap();
    assert!(matches!(
        storage.promote(&staged),
        Err(StorageError::AlreadyExists)
    ));
    assert_eq!(fs::read(&existing).unwrap(), b"existing");
}

#[test]
fn space_precheck_requires_planned_size_plus_reserve() {
    let fx = fixture();
    let source_path = write_source(&fx.source_dir, "a.m4a", &synthetic(SIZE));
    let limits = CopyLimits {
        reserve_free_bytes: 1_000_000,
        ..small_limits()
    };
    let required = SIZE as u64 + limits.reserve_free_bytes;

    let (storage, _) = faulty(&fx, FaultyIo::with_space(&[required - 1]));
    let error = storage
        .copy_checked(
            &mut FileSource::open(&source_path).unwrap(),
            storage.allocate_staging(track()).unwrap(),
            &limits,
            &not_cancelled(),
        )
        .unwrap_err();
    assert!(
        matches!(error, StorageError::StorageFull { .. }),
        "{error:?}"
    );
    assert_eq!(error.reason(), "storage_full");
    assert!(error.retryable());
    assert!(names(&fx.data_root.join("staging")).is_empty());

    let (storage, _) = faulty(&fx, FaultyIo::with_space(&[required]));
    storage
        .copy_checked(
            &mut FileSource::open(&source_path).unwrap(),
            storage.allocate_staging(track()).unwrap(),
            &limits,
            &not_cancelled(),
        )
        .unwrap();
}

#[test]
fn space_shrinking_during_copy_aborts_and_cleans_up() {
    let fx = fixture();
    let source_path = write_source(&fx.source_dir, "a.m4a", &synthetic(SIZE));
    let limits = CopyLimits {
        reserve_free_bytes: 1_000_000,
        ..small_limits()
    };
    // 開始時は十分、途中で他プロセスが容量を使い下限を割る。
    let (storage, io) = faulty(&fx, FaultyIo::with_space(&[u64::MAX, 1_000_100]));
    let error = storage
        .copy_checked(
            &mut FileSource::open(&source_path).unwrap(),
            storage.allocate_staging(track()).unwrap(),
            &limits,
            &not_cancelled(),
        )
        .unwrap_err();
    assert!(
        matches!(error, StorageError::StorageFull { .. }),
        "{error:?}"
    );
    assert!(io.space_calls.load(Ordering::SeqCst) >= 2);
    assert!(names(&fx.data_root.join("staging")).is_empty());
}

#[test]
fn size_limit_applies_before_and_during_copy() {
    let fx = fixture();
    let storage = Storage::open(&fx.data_root).unwrap();
    let source_path = write_source(&fx.source_dir, "big.m4a", &synthetic(SIZE));
    let limits = CopyLimits {
        max_file_bytes: SIZE as u64 - 1,
        ..small_limits()
    };
    let error = storage
        .copy_checked(
            &mut FileSource::open(&source_path).unwrap(),
            storage.allocate_staging(track()).unwrap(),
            &limits,
            &not_cancelled(),
        )
        .unwrap_err();
    assert!(matches!(error, StorageError::FileTooLarge { .. }));
    assert_eq!(error.reason(), "file_too_large");
    assert!(!error.retryable());

    // 属性上は小さくても、実際の読み出しが上限を超えたら止める。
    let mut lying = LyingSize {
        inner: FileSource::open(&source_path).unwrap(),
        claimed: 1000,
    };
    let limits = CopyLimits {
        max_file_bytes: 2000,
        ..small_limits()
    };
    let error = storage
        .copy_checked(
            &mut lying,
            storage.allocate_staging(track()).unwrap(),
            &limits,
            &not_cancelled(),
        )
        .unwrap_err();
    assert!(
        matches!(error, StorageError::FileTooLarge { .. }),
        "{error:?}"
    );
    assert!(names(&fx.data_root.join("staging")).is_empty());
}

/// 属性のサイズを偽るコピー元（読み出し量の上限検査用）。
struct LyingSize {
    inner: FileSource,
    claimed: u64,
}

impl Read for LyingSize {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read(buf)
    }
}
impl Seek for LyingSize {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.inner.seek(pos)
    }
}
impl CopySource for LyingSize {
    fn handle_snapshot(&self) -> io::Result<SourceSnapshot> {
        Ok(SourceSnapshot {
            size: self.claimed,
            ..self.inner.handle_snapshot()?
        })
    }
    fn entry_snapshot(&self) -> io::Result<SourceSnapshot> {
        Ok(SourceSnapshot {
            size: self.claimed,
            ..self.inner.entry_snapshot()?
        })
    }
}

/// 最初の読み出し後に元ファイルへ追記するコピー元（書き込み中の元を再現）。
struct GrowingSource {
    inner: FileSource,
    path: PathBuf,
    appended: bool,
}

impl Read for GrowingSource {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let read = self.inner.read(buf)?;
        if !self.appended {
            self.appended = true;
            let mut file = fs::OpenOptions::new().append(true).open(&self.path)?;
            file.write_all(b"more data")?;
        }
        Ok(read)
    }
}
impl Seek for GrowingSource {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.inner.seek(pos)
    }
}
impl CopySource for GrowingSource {
    fn handle_snapshot(&self) -> io::Result<SourceSnapshot> {
        self.inner.handle_snapshot()
    }
    fn entry_snapshot(&self) -> io::Result<SourceSnapshot> {
        self.inner.entry_snapshot()
    }
}

#[test]
fn source_modified_during_copy_is_unstable() {
    let fx = fixture();
    let storage = Storage::open(&fx.data_root).unwrap();
    let path = write_source(&fx.source_dir, "growing.m4a", &synthetic(SIZE));
    let mut source = GrowingSource {
        inner: FileSource::open(&path).unwrap(),
        path: path.clone(),
        appended: false,
    };
    let error = storage
        .copy_checked(
            &mut source,
            storage.allocate_staging(track()).unwrap(),
            &small_limits(),
            &not_cancelled(),
        )
        .unwrap_err();
    assert!(matches!(error, StorageError::FileUnstable), "{error:?}");
    assert_eq!(error.reason(), "file_unstable");
    assert!(names(&fx.data_root.join("staging")).is_empty());
}

#[test]
fn source_entry_replaced_or_removed_after_open_is_unstable() {
    let fx = fixture();
    let storage = Storage::open(&fx.data_root).unwrap();
    let path = write_source(&fx.source_dir, "a.m4a", &synthetic(SIZE));
    let mut source = FileSource::open(&path).unwrap();
    let other = write_source(&fx.source_dir, "other.m4a", &synthetic(SIZE));
    fs::rename(&other, &path).unwrap();
    let error = storage
        .copy_checked(
            &mut source,
            storage.allocate_staging(track()).unwrap(),
            &small_limits(),
            &not_cancelled(),
        )
        .unwrap_err();
    assert!(matches!(error, StorageError::FileUnstable));

    let mut source = FileSource::open(&path).unwrap();
    fs::remove_file(&path).unwrap();
    let error = storage
        .copy_checked(
            &mut source,
            storage.allocate_staging(track()).unwrap(),
            &small_limits(),
            &not_cancelled(),
        )
        .unwrap_err();
    assert!(matches!(error, StorageError::FileUnstable));
    assert!(names(&fx.data_root.join("staging")).is_empty());
}

#[test]
fn cancellation_discards_staging() {
    let fx = fixture();
    let storage = Storage::open(&fx.data_root).unwrap();
    let path = write_source(&fx.source_dir, "a.m4a", &synthetic(SIZE));
    let cancel = AtomicBool::new(true);
    let error = storage
        .copy_checked(
            &mut FileSource::open(&path).unwrap(),
            storage.allocate_staging(track()).unwrap(),
            &small_limits(),
            &cancel,
        )
        .unwrap_err();
    assert!(matches!(error, StorageError::Cancelled));
    assert_eq!(error.reason(), "interrupted");
    assert!(names(&fx.data_root.join("staging")).is_empty());
}

#[test]
fn unused_staging_is_removed_on_drop() {
    let fx = fixture();
    let storage = Storage::open(&fx.data_root).unwrap();
    let staging = storage.allocate_staging(track()).unwrap();
    assert_eq!(names(&fx.data_root.join("staging")).len(), 1);
    drop(staging);
    assert!(names(&fx.data_root.join("staging")).is_empty());
}

#[test]
fn remove_managed_is_idempotent_and_keeps_state_on_failure() {
    let fx = fixture();
    let (storage, io) = faulty(&fx, FaultyIo::default());
    let staged = staged_with(&fx, &storage);
    let media = storage.promote(&staged).unwrap();
    let target = ManagedFile::Media(media.track_id);

    io.fail_remove.store(true, Ordering::SeqCst);
    assert!(matches!(
        storage.remove_managed(target),
        Err(StorageError::Io(_))
    ));
    assert!(storage.path_of(target).exists());

    io.fail_remove.store(false, Ordering::SeqCst);
    storage.remove_managed(target).unwrap();
    assert!(!storage.path_of(target).exists());
    // 既にない場合は成功。
    storage.remove_managed(target).unwrap();
    storage
        .remove_managed(ManagedFile::Staging(Uuid::new_v4()))
        .unwrap();
}

#[test]
fn remove_managed_only_touches_managed_names() {
    let fx = fixture();
    let storage = Storage::open(&fx.data_root).unwrap();
    // 管理用ディレクトリ内の無関係なファイルや外部ファイルは列挙・削除の対象外。
    let foreign = fx.data_root.join("media").join("曲名.m4a");
    fs::write(&foreign, b"x").unwrap();
    let upper = fx
        .data_root
        .join("media")
        .join(format!("{}.m4a", Uuid::new_v4().hyphenated()).to_uppercase());
    fs::write(&upper, b"x").unwrap();
    fs::create_dir(
        fx.data_root
            .join("media")
            .join(format!("{}.m4a", Uuid::new_v4())),
    )
    .unwrap();
    assert!(storage.list_managed(ManagedKind::Media).unwrap().is_empty());
    let outside = write_source(&fx.source_dir, "keep.m4a", b"keep");
    for kind in [
        ManagedKind::Staging,
        ManagedKind::Media,
        ManagedKind::Artwork,
    ] {
        for file in storage.list_managed(kind).unwrap() {
            storage.remove_managed(file).unwrap();
        }
    }
    assert!(foreign.exists() && upper.exists() && outside.exists());

    // 管理名のディレクトリは削除しない。
    let dir_id = TrackId::new(Uuid::new_v4()).unwrap();
    fs::create_dir(storage.path_of(ManagedFile::Media(dir_id))).unwrap();
    assert!(matches!(
        storage.remove_managed(ManagedFile::Media(dir_id)),
        Err(StorageError::InvalidTarget)
    ));
}

#[test]
fn quarantine_moves_unreferenced_media_out_of_media_dir() {
    let fx = fixture();
    let storage = Storage::open(&fx.data_root).unwrap();
    let staged = staged_with(&fx, &storage);
    storage.promote(&staged).unwrap();
    let moved = storage.quarantine(staged.target()).unwrap();
    assert!(moved.relative_path.starts_with("quarantine/"));
    assert!(
        moved
            .relative_path
            .ends_with(&format!("{}.m4a", staged.track_id().as_uuid()))
    );
    assert!(storage.list_managed(ManagedKind::Media).unwrap().is_empty());
    let kept = fs::read(fx.data_root.join(&moved.relative_path)).unwrap();
    assert_eq!(kept, synthetic(SIZE));
    assert!(storage.quarantine(staged.target()).is_err());
}

#[test]
fn open_rejects_symlinked_or_file_subdirectories() {
    let fx = fixture();
    std::os::unix::fs::symlink(&fx.source_dir, fx.data_root.join("media")).unwrap();
    assert!(matches!(
        Storage::open(&fx.data_root),
        Err(StorageError::InvalidLayout(_))
    ));

    let fx = fixture();
    fs::write(fx.data_root.join("staging"), b"").unwrap();
    assert!(matches!(
        Storage::open(&fx.data_root),
        Err(StorageError::InvalidLayout(_))
    ));
}

#[test]
fn digest_hex_roundtrip_and_rejects_noncanonical() {
    let digest = FileDigest::new([0xab; 32]);
    let hex = digest.to_hex();
    assert_eq!(hex.len(), 64);
    assert_eq!(FileDigest::from_hex(&hex), Some(digest));
    assert_eq!(FileDigest::from_hex(&hex.to_uppercase()), None);
    assert_eq!(FileDigest::from_hex(&hex[..62]), None);
}
