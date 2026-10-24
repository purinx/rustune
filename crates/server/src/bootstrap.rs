//! 起動・終了の共通基盤。recovery・listener・ワーカーの接続は後続タスクで行う。
//! DBの作成に成功しただけではライブラリのready状態とはみなさない。
use crate::{
    config::{self, ConfigError, RuntimeConfig, SourceStatus},
    db::{
        DbError, Repository,
        settings::{LibrarySettings, stored_source_path},
    },
};
use std::{
    fs::File,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use tokio::sync::Notify;
use uuid::Uuid;

#[derive(Debug)]
pub enum BootstrapError {
    Config(ConfigError),
    Database(DbError),
    StorageUnavailable,
    AlreadyRunning,
    LockUnavailable,
}
impl std::fmt::Display for BootstrapError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Config(_) => "invalid configuration",
            Self::Database(_) => "database startup failed",
            Self::StorageUnavailable => "private storage unavailable",
            Self::AlreadyRunning => "Rustune is already running",
            Self::LockUnavailable => "process lock unavailable",
        })
    }
}
impl std::error::Error for BootstrapError {}
impl From<ConfigError> for BootstrapError {
    fn from(e: ConfigError) -> Self {
        Self::Config(e)
    }
}
impl From<DbError> for BootstrapError {
    fn from(e: DbError) -> Self {
        Self::Database(e)
    }
}

/// ワーカーとSQLiteを閉じ終えるまでこのハンドルを保持する。ロックファイルは削除しない。
/// 削除すると新しいinodeに対して別プロセスが二つ目のロックを取得できてしまう。
pub struct ProcessLock {
    _file: File,
}
impl ProcessLock {
    pub fn acquire(directory: &Path) -> Result<Self, BootstrapError> {
        let file = rustix::fs::open(
            directory.join("rustune.lock"),
            rustix::fs::OFlags::RDWR
                | rustix::fs::OFlags::CREATE
                | rustix::fs::OFlags::CLOEXEC
                | rustix::fs::OFlags::NOFOLLOW,
            rustix::fs::Mode::RUSR | rustix::fs::Mode::WUSR,
        )
        .map_err(|_| BootstrapError::LockUnavailable)?;
        match rustix::fs::flock(&file, rustix::fs::FlockOperation::NonBlockingLockExclusive) {
            Ok(()) => Ok(Self {
                _file: File::from(file),
            }),
            Err(e) if e == rustix::io::Errno::WOULDBLOCK => Err(BootstrapError::AlreadyRunning),
            Err(_) => Err(BootstrapError::LockUnavailable),
        }
    }
}

pub struct BootstrapContext {
    pub config: RuntimeConfig,
    pub repository: Repository,
    pub source_status: SourceStatus,
    lock: ProcessLock,
    recovered: bool,
}
impl BootstrapContext {
    pub fn ready(&self) -> bool {
        self.recovered
    }
    /// タスク09の起動時recoveryが成功した後にだけ呼び出す。
    pub fn mark_recovered(&mut self) {
        self.recovered = true;
    }
    pub async fn shutdown(self) {
        self.repository.close().await;
        drop(self.lock);
    }
}
pub async fn bootstrap(
    config: RuntimeConfig,
    initial_source: &Path,
) -> Result<BootstrapContext, BootstrapError> {
    config::reject_overlapping_roots(initial_source, &config.data_dir)?;
    let db_path = config.data_dir.join("library.sqlite3");
    // 既存DBがあれば、保存先の準備（ディレクトリ作成・権限変更）とmigrationより前に、
    // 保存済みの取り込み元との重なりを確かめる。重なっていたら、取り込み元の配下へ何も書き込まずに止める。
    if let Some(stored) = stored_source_path(&db_path).await? {
        config::reject_overlapping_roots(&path_from_bytes(stored), &config.data_dir)?;
    }
    prepare_storage(&config.data_dir)?;
    let lock = ProcessLock::acquire(&config.data_dir)?;
    let repository = Repository::open(&db_path).await?;
    let defaults =
        LibrarySettings::defaults(Uuid::new_v4(), path_bytes(initial_source), "Music".into());
    if let Err(error) = repository.initialize_settings(&defaults).await {
        repository.close().await;
        return Err(error.into());
    }
    let stored = match repository.settings().await {
        Ok(stored) => stored,
        Err(error) => {
            repository.close().await;
            return Err(error.into());
        }
    };
    // ロック取得前に読んだ値から変わっていても安全なよう、開いたDBの値でもう一度確かめる。
    let source = path_from_bytes(stored.source_path_bytes);
    if let Err(error) = config::reject_overlapping_roots(&source, &config.data_dir) {
        repository.close().await;
        return Err(error.into());
    }
    Ok(BootstrapContext {
        config,
        repository,
        source_status: config::inspect_source_root(&source),
        lock,
        recovered: false,
    })
}
#[cfg(unix)]
fn path_bytes(path: &Path) -> Vec<u8> {
    use std::os::unix::ffi::OsStrExt;
    path.as_os_str().as_bytes().to_vec()
}
#[cfg(unix)]
fn path_from_bytes(bytes: Vec<u8>) -> PathBuf {
    use std::os::unix::ffi::OsStringExt;
    std::ffi::OsString::from_vec(bytes).into()
}
fn prepare_storage(path: &Path) -> Result<(), BootstrapError> {
    use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
    if let Ok(meta) = std::fs::symlink_metadata(path)
        && (meta.file_type().is_symlink() || !meta.is_dir())
    {
        return Err(BootstrapError::StorageUnavailable);
    }
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(path)
        .map_err(|_| BootstrapError::StorageUnavailable)?;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))
        .map_err(|_| BootstrapError::StorageUnavailable)?;
    // SQLiteやrecoveryの前に書き込み可否を確認する。取り込み元ファイルには触れない。
    let probe = path.join(format!(".write-check-{}", Uuid::new_v4()));
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&probe)
        .map_err(|_| BootstrapError::StorageUnavailable)?;
    drop(file);
    std::fs::remove_file(probe).map_err(|_| BootstrapError::StorageUnavailable)?;
    for name in ["media", "artwork", "staging"] {
        let child = path.join(name);
        if let Ok(meta) = std::fs::symlink_metadata(&child)
            && (meta.file_type().is_symlink() || !meta.is_dir())
        {
            return Err(BootstrapError::StorageUnavailable);
        }
        std::fs::DirBuilder::new()
            .mode(0o700)
            .create(&child)
            .or_else(|e| {
                if e.kind() == std::io::ErrorKind::AlreadyExists {
                    Ok(())
                } else {
                    Err(e)
                }
            })
            .map_err(|_| BootstrapError::StorageUnavailable)?;
        std::fs::set_permissions(child, std::fs::Permissions::from_mode(0o700))
            .map_err(|_| BootstrapError::StorageUnavailable)?;
    }
    Ok(())
}

#[derive(Clone, Default)]
pub struct Shutdown {
    inner: Arc<ShutdownInner>,
}
#[derive(Default)]
struct ShutdownInner {
    requested: AtomicBool,
    notify: Notify,
}
impl Shutdown {
    pub fn request(&self) {
        self.inner.requested.store(true, Ordering::Release);
        self.inner.notify.notify_waiters();
    }
    pub fn is_requested(&self) -> bool {
        self.inner.requested.load(Ordering::Acquire)
    }
    pub async fn cancelled(&self) {
        loop {
            let notified = self.inner.notify.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            if self.is_requested() {
                return;
            }
            notified.await;
        }
    }
    /// 先に新規予約を止め、呼び出し側は協調キャンセル中のワーカー完了待ちを渡す。
    /// falseは未完了の処理が残り、次回起動時のrecoveryで処理することを意味する。
    pub async fn drain(&self, workers: impl std::future::Future<Output = ()>) -> bool {
        self.drain_within(SHUTDOWN_DRAIN_TIMEOUT, workers).await
    }
    /// 待ち時間の上限を明示する[`Shutdown::drain`]。テストや運用ツール向け。
    pub async fn drain_within(
        &self,
        bound: Duration,
        workers: impl std::future::Future<Output = ()>,
    ) -> bool {
        self.request();
        tokio::time::timeout(bound, workers).await.is_ok()
    }
}
/// SIGTERM後に協調キャンセルを待つ最大時間（MODULE_DESIGN §4）。
pub const SHUTDOWN_DRAIN_TIMEOUT: Duration = Duration::from_secs(10);
/// 生成時にOSのシグナルハンドラを登録する終了待ち受け。`install()`が返った後に
/// 届いたシグナルが既定動作（プロセス終了）で失われることはない。
pub struct ShutdownSignal {
    #[cfg(unix)]
    terminate: tokio::signal::unix::Signal,
    #[cfg(unix)]
    interrupt: tokio::signal::unix::Signal,
}
impl ShutdownSignal {
    /// Tokioランタイム内で呼び出すこと。
    pub fn install() -> std::io::Result<Self> {
        #[cfg(unix)]
        {
            use tokio::signal::unix::{SignalKind, signal};
            Ok(Self {
                terminate: signal(SignalKind::terminate())?,
                interrupt: signal(SignalKind::interrupt())?,
            })
        }
        #[cfg(not(unix))]
        {
            Ok(Self {})
        }
    }
    /// 最初のSIGTERMまたはSIGINT（Ctrl-C）で完了する。
    pub async fn recv(mut self) -> std::io::Result<()> {
        #[cfg(unix)]
        {
            tokio::select! {
                _ = self.terminate.recv() => Ok(()),
                _ = self.interrupt.recv() => Ok(()),
            }
        }
        #[cfg(not(unix))]
        {
            tokio::signal::ctrl_c().await
        }
    }
}
/// ハンドラを登録してSIGTERM/SIGINTを待つ簡易関数。
pub async fn shutdown_signal() -> std::io::Result<()> {
    ShutdownSignal::install()?.recv().await
}
