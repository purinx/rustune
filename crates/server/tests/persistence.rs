use rustune_server::{
    bootstrap::{self, BootstrapError, ProcessLock, Shutdown},
    config::{self, ConfigError, SourceStatus},
    db::{DbError, Repository, settings::LibrarySettings},
};
use sqlx::Row;
use uuid::Uuid;

async fn repo() -> (tempfile::TempDir, Repository) {
    let dir = tempfile::tempdir().unwrap();
    let repo = Repository::open(&dir.path().join("test.sqlite3"))
        .await
        .unwrap();
    let settings =
        LibrarySettings::defaults(Uuid::new_v4(), b"/fixture/source".to_vec(), "Music".into());
    repo.initialize_settings(&settings).await.unwrap();
    (dir, repo)
}

#[tokio::test]
async fn migration_pragmas_and_restart_preserve_state() {
    let (dir, r) = repo().await;
    let journal: String = sqlx::query_scalar("PRAGMA journal_mode")
        .fetch_one(r.pool())
        .await
        .unwrap();
    assert_eq!(journal, "wal");
    let foreign: i64 = sqlx::query_scalar("PRAGMA foreign_keys")
        .fetch_one(r.pool())
        .await
        .unwrap();
    assert_eq!(foreign, 1);
    let busy: i64 = sqlx::query_scalar("PRAGMA busy_timeout")
        .fetch_one(r.pool())
        .await
        .unwrap();
    assert_eq!(busy, 5000);
    let id = Uuid::new_v4();
    r.create_playlist(id, "  お気に入り  ").await.unwrap();
    let original = r.settings().await.unwrap();
    r.close().await;
    let reopened = Repository::open(&dir.path().join("test.sqlite3"))
        .await
        .unwrap();
    assert_eq!(reopened.settings().await.unwrap(), original);
    assert_eq!(reopened.library_revision().await.unwrap(), 1);
    let name: String = sqlx::query_scalar("SELECT name FROM playlists WHERE id=?")
        .bind(id.to_string())
        .fetch_one(reopened.pool())
        .await
        .unwrap();
    assert_eq!(name, "お気に入り");
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM _sqlx_migrations")
        .fetch_one(reopened.pool())
        .await
        .unwrap();
    assert_eq!(count, 1);
    reopened.close().await;
}

#[tokio::test]
async fn constraints_and_repository_errors_rollback_without_revision_increment() {
    let (_dir, r) = repo().await;
    let id = Uuid::new_v4();
    r.create_playlist(id, "first").await.unwrap();
    assert!(r.create_playlist(id, "duplicate").await.is_err());
    assert_eq!(r.library_revision().await.unwrap(), 1);
    assert!(
        sqlx::query("INSERT INTO favorites(track_id) VALUES(?)")
            .bind(Uuid::new_v4().to_string())
            .execute(r.pool())
            .await
            .is_err()
    );
    assert!(
        sqlx::query(
            "INSERT INTO tracks(id,state,created_at) VALUES(?,'nonsense','2026-10-18T00:00:00Z')"
        )
        .bind(Uuid::new_v4().to_string())
        .execute(r.pool())
        .await
        .is_err()
    );
    assert!(
        sqlx::query(
            "INSERT INTO tracks(id,state,created_at) VALUES(?,'ready','2026-10-18T00:00:00Z')"
        )
        .bind(Uuid::new_v4().to_string())
        .execute(r.pool())
        .await
        .is_err()
    );
    let track = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO tracks(id,state,created_at) VALUES(?,'staging','2026-10-18T00:00:00Z')",
    )
    .bind(&track)
    .execute(r.pool())
    .await
    .unwrap();
    sqlx::query("INSERT INTO identities(scheme,value,track_id) VALUES('store',x'01',?)")
        .bind(&track)
        .execute(r.pool())
        .await
        .unwrap();
    assert!(
        sqlx::query("INSERT INTO identities(scheme,value,track_id) VALUES('store',x'01',?)")
            .bind(&track)
            .execute(r.pool())
            .await
            .is_err()
    );
    sqlx::query("INSERT INTO exclusions(track_id,excluded_at) VALUES(?,'2026-10-18T00:00:00Z')")
        .bind(&track)
        .execute(r.pool())
        .await
        .unwrap();
    assert!(
        sqlx::query("DELETE FROM tracks WHERE id=?")
            .bind(&track)
            .execute(r.pool())
            .await
            .is_err()
    );
    // 更新後の失敗でRepository操作全体がrollbackされる。
    sqlx::query("UPDATE library_state SET library_revision=9223372036854775807")
        .execute(r.pool())
        .await
        .unwrap();
    assert!(matches!(
        r.rename_playlist(id, 0, "must rollback").await,
        Err(DbError::Conflict)
    ));
    let row = sqlx::query("SELECT name,revision FROM playlists WHERE id=?")
        .bind(id.to_string())
        .fetch_one(r.pool())
        .await
        .unwrap();
    assert_eq!(row.get::<String, _>("name"), "first");
    assert_eq!(row.get::<i64, _>("revision"), 0);
}

#[tokio::test]
async fn concurrent_cas_has_one_winner_and_no_lost_updates() {
    let (_dir, r) = repo().await;
    let id = Uuid::new_v4();
    r.create_playlist(id, "original").await.unwrap();
    let (a, b) = tokio::join!(r.rename_playlist(id, 0, "a"), r.rename_playlist(id, 0, "b"));
    assert_eq!(usize::from(a.is_ok()) + usize::from(b.is_ok()), 1);
    assert!(matches!(a, Err(DbError::Conflict)) || matches!(b, Err(DbError::Conflict)));
    assert_eq!(r.library_revision().await.unwrap(), 2);
}

#[tokio::test]
async fn import_requests_merge_and_settings_root_updates_are_atomic() {
    let (_dir, r) = repo().await;
    let first = Uuid::new_v4();
    let second = Uuid::new_v4();
    let (a, b) = tokio::join!(
        r.reserve_import(first, "manual", "2026-10-18T00:00:00Z"),
        r.reserve_import(second, "startup", "2026-10-18T00:00:00Z")
    );
    assert_eq!(a.unwrap(), b.unwrap());
    let mut settings = r.settings().await.unwrap();
    let old = settings.clone();
    settings.source_root_id = Uuid::new_v4();
    settings.source_path_bytes = b"/new/source".to_vec();
    assert!(matches!(
        r.update_settings(0, &settings).await,
        Err(DbError::Conflict)
    ));
    assert_eq!(r.settings().await.unwrap(), old);
    // 自動取り込みを無効にしても既存のjobは取り消さない。
    let mut disable = old.clone();
    disable.auto_import = false;
    assert_eq!(r.update_settings(0, &disable).await.unwrap(), 1);
    let jobs: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM import_jobs WHERE state='queued'")
        .fetch_one(r.pool())
        .await
        .unwrap();
    assert_eq!(jobs, 1);
    sqlx::query("UPDATE import_jobs SET state='completed',finished_at='2026-10-18T00:01:00Z'")
        .execute(r.pool())
        .await
        .unwrap();
    assert_eq!(r.update_settings(1, &settings).await.unwrap(), 2);
    let roots: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM source_roots")
        .fetch_one(r.pool())
        .await
        .unwrap();
    assert_eq!(roots, 2);
    let active: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM source_roots WHERE active=1")
        .fetch_one(r.pool())
        .await
        .unwrap();
    assert_eq!(active, 1);
    assert!(matches!(
        r.update_settings(1, &settings).await,
        Err(DbError::Conflict)
    ));
}

#[test]
fn runtime_settings_reject_unknown_invalid_values_and_overlapping_roots() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path();
    let config = config::load_config("", home).unwrap();
    assert_eq!(config.address.port(), 3867);
    for source in [
        "port=0",
        "bind='0.0.0.0'",
        "auto_import=true",
        "interval_seconds=60",
        "data_dir='relative'",
        "port=999999",
    ] {
        assert!(config::load_config(source, home).is_err(), "{source}");
    }
    let source = home.join("source");
    let storage = home.join("storage");
    assert!(config::validate_separate_roots(&source, &storage).is_ok());
    assert_eq!(
        config::validate_separate_roots(&source, &source.join("nested")),
        Err(ConfigError::OverlappingRoots)
    );
    assert_eq!(
        config::validate_separate_roots(&storage, &source.join("../storage")),
        Err(ConfigError::InvalidPath)
    );
    std::fs::create_dir(&source).unwrap();
    std::os::unix::fs::symlink(&source, &storage).unwrap();
    assert_eq!(
        config::validate_separate_roots(&source, &storage),
        Err(ConfigError::OverlappingRoots)
    );
    assert_eq!(
        config::inspect_source_root(&storage),
        SourceStatus::RejectedSymlink
    );
}

#[test]
fn web_dist_must_be_absolute_and_defaults_next_to_the_binary() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path();
    for source in [
        "web_dist='relative'",
        "web_dist='../assets'",
        "web_dist='/srv/../assets'",
        "web_dist=''",
    ] {
        assert_eq!(
            config::load_config(source, home),
            Err(ConfigError::InvalidPath),
            "{source}"
        );
    }
    let explicit = config::load_config("web_dist='/opt/rustune/web'", home).unwrap();
    assert_eq!(
        explicit.web_dist,
        std::path::PathBuf::from("/opt/rustune/web")
    );
    let release = std::path::Path::new("/Applications/Rustune/bin");
    let defaulted = config::load_config_with_release_dir("", home, release).unwrap();
    assert_eq!(defaulted.web_dist, release.join(config::RELEASE_WEB_DIR));
    let from_exe = config::load_config("", home).unwrap();
    assert!(from_exe.web_dist.is_absolute());
    assert!(from_exe.web_dist.ends_with(config::RELEASE_WEB_DIR));
    assert_eq!(
        config::load_config_with_release_dir("", home, std::path::Path::new("relative/bin")),
        Err(ConfigError::InvalidPath)
    );
}

#[test]
fn overlap_through_alias_is_detected_even_below_unreadable_directory() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let data = dir.path().join("data");
    let private = data.join("private");
    std::fs::create_dir_all(private.join("music")).unwrap();
    let alias = dir.path().join("alias");
    std::os::unix::fs::symlink(&data, &alias).unwrap();
    // 存在しない成分だけが残る場合も別名を解決して重なりを検出する。
    assert_eq!(
        config::validate_separate_roots(&alias.join("missing/music"), &data),
        Err(ConfigError::OverlappingRoots)
    );
    std::fs::set_permissions(&private, std::fs::Permissions::from_mode(0o000)).unwrap();
    // 指摘の例：/alias -> /data、/data/privateは読めない、source=/alias/private/music。
    let source = alias.join("private/music");
    assert_eq!(
        config::root_relation(&source, &data),
        Ok(config::RootRelation::Overlapping)
    );
    assert_eq!(
        config::validate_separate_roots(&source, &data),
        Err(ConfigError::OverlappingRoots)
    );
    assert_eq!(
        config::reject_overlapping_roots(&source, &data),
        Err(ConfigError::OverlappingRoots)
    );
    // 保存先側が別名経由でも同じ。
    assert_eq!(
        config::validate_separate_roots(&private.join("music"), &alias),
        Err(ConfigError::OverlappingRoots)
    );
    std::fs::set_permissions(&private, std::fs::Permissions::from_mode(0o700)).unwrap();
}

#[test]
fn overlap_through_symlink_to_missing_target_is_detected() {
    let dir = tempfile::tempdir().unwrap();
    let data = dir.path().join("data");
    // 指摘の例：alias -> data（dataは未作成）、取り込み元alias/media、保存先data。
    let alias = dir.path().join("alias");
    std::os::unix::fs::symlink("data", &alias).unwrap();
    let source = alias.join("media");
    assert_eq!(
        config::root_relation(&source, &data),
        Ok(config::RootRelation::Overlapping)
    );
    assert_eq!(
        config::validate_separate_roots(&source, &data),
        Err(ConfigError::OverlappingRoots)
    );
    assert_eq!(
        config::reject_overlapping_roots(&source, &data),
        Err(ConfigError::OverlappingRoots)
    );
    // 絶対パスのリンクと、`..`を含む相対リンクも同じ。
    let absolute = dir.path().join("absolute");
    std::os::unix::fs::symlink(&data, &absolute).unwrap();
    assert_eq!(
        config::root_relation(&absolute.join("media"), &data),
        Ok(config::RootRelation::Overlapping)
    );
    let nested = dir.path().join("nested");
    std::fs::create_dir(&nested).unwrap();
    std::os::unix::fs::symlink("../data", nested.join("up")).unwrap();
    assert_eq!(
        config::root_relation(&nested.join("up/media"), &data),
        Ok(config::RootRelation::Overlapping)
    );
    // 別の未作成の場所を指すリンクは重ならない。
    let elsewhere = dir.path().join("elsewhere");
    std::os::unix::fs::symlink("other", &elsewhere).unwrap();
    assert_eq!(
        config::root_relation(&elsewhere.join("media"), &data),
        Ok(config::RootRelation::Separate)
    );
    // 循環するリンクは解決できないものとして拒否する。
    std::os::unix::fs::symlink("loop-b", dir.path().join("loop-a")).unwrap();
    std::os::unix::fs::symlink("loop-a", dir.path().join("loop-b")).unwrap();
    assert_eq!(
        config::root_relation(&dir.path().join("loop-a/media"), &data),
        Err(ConfigError::Io)
    );
    // 判定だけで、リンク先のディレクトリを作らない。
    assert!(!data.exists());
}

#[tokio::test]
async fn stored_source_overlap_is_rejected_before_storage_is_prepared() {
    use std::os::unix::{ffi::OsStrExt, fs::PermissionsExt};
    let dir = tempfile::tempdir().unwrap();
    // 指摘の例：DBを保存済み取り込み元の配下へ移し、data_dirをそこへ変更した。
    let stored_source = dir.path().join("music");
    let data = stored_source.join("rustune-data");
    std::fs::create_dir_all(&data).unwrap();
    let repo = Repository::open(&data.join("library.sqlite3"))
        .await
        .unwrap();
    let settings = LibrarySettings::defaults(
        Uuid::new_v4(),
        stored_source.as_os_str().as_bytes().to_vec(),
        "Music".into(),
    );
    repo.initialize_settings(&settings).await.unwrap();
    repo.close().await;
    std::fs::set_permissions(&data, std::fs::Permissions::from_mode(0o755)).unwrap();
    let config =
        config::load_config(&format!("data_dir = '{}'", data.display()), dir.path()).unwrap();
    // 起動時に渡す取り込み元（初回用の既定値）は、保存先と重ならない別の場所。
    let initial_source = dir.path().join("other");
    assert!(matches!(
        bootstrap::bootstrap(config, &initial_source).await,
        Err(BootstrapError::Config(ConfigError::OverlappingRoots))
    ));
    // 保存先の準備（サブディレクトリ作成・ロック・権限変更）を行う前に止まっている。
    for name in ["media", "artwork", "staging", "rustune.lock"] {
        assert!(!data.join(name).exists(), "{name}");
    }
    let mode = std::fs::metadata(&data).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o755);
}

#[tokio::test]
async fn unverifiable_source_is_not_confirmed_separate_but_startup_continues() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let locked = dir.path().join("locked");
    std::fs::create_dir_all(locked.join("inner/music")).unwrap();
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000)).unwrap();
    // rootユーザーは権限を無視できるため、その場合は解決でき分離と判定される。
    let privileged = std::fs::read_dir(&locked).is_ok();
    let source = locked.join("inner/music");
    let config =
        config::load_config_with_release_dir("", dir.path(), std::path::Path::new("/opt/rustune"))
            .unwrap();
    let relation = config::root_relation(&source, &config.data_dir).unwrap();
    if privileged {
        assert_eq!(relation, config::RootRelation::Separate);
    } else {
        assert_eq!(relation, config::RootRelation::Unverifiable);
        assert_eq!(
            config::validate_separate_roots(&source, &config.data_dir),
            Err(ConfigError::UnverifiableRoots)
        );
    }
    let app = bootstrap::bootstrap(config, &source).await.unwrap();
    if !privileged {
        assert_eq!(app.source_status, SourceStatus::PermissionDenied);
    }
    assert!(!app.ready());
    app.shutdown().await;
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o700)).unwrap();
}

#[tokio::test]
async fn bootstrap_missing_source_stays_not_ready_and_lock_excludes_second_process() {
    let dir = tempfile::tempdir().unwrap();
    let config = config::load_config("", dir.path()).unwrap();
    let missing = dir.path().join("missing");
    let mut app = bootstrap::bootstrap(config.clone(), &missing)
        .await
        .unwrap();
    assert_eq!(app.source_status, SourceStatus::Missing);
    assert!(!app.ready());
    assert!(matches!(
        ProcessLock::acquire(&config.data_dir),
        Err(BootstrapError::AlreadyRunning)
    ));
    app.mark_recovered();
    assert!(app.ready());
    app.shutdown().await;
    let app = bootstrap::bootstrap(config, &missing).await.unwrap();
    assert!(!app.ready());
    app.shutdown().await;
}

#[tokio::test]
async fn unreadable_source_does_not_prevent_database_startup_and_storage_file_is_rejected() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("source");
    std::fs::create_dir(&source).unwrap();
    std::fs::set_permissions(&source, std::fs::Permissions::from_mode(0o000)).unwrap();
    let status = config::inspect_source_root(&source);
    // rootユーザーはPOSIX権限を無視できるが、通常のデスクトップユーザーはできない。
    assert!(matches!(
        status,
        SourceStatus::PermissionDenied | SourceStatus::Available
    ));
    let config = config::load_config("", dir.path()).unwrap();
    let app = bootstrap::bootstrap(config, &source).await.unwrap();
    assert_eq!(app.source_status, status);
    app.shutdown().await;
    std::fs::set_permissions(&source, std::fs::Permissions::from_mode(0o700)).unwrap();
    let storage = dir.path().join("file");
    std::fs::write(&storage, b"not a directory").unwrap();
    let config =
        config::load_config(&format!("data_dir = '{}'", storage.display()), dir.path()).unwrap();
    assert!(matches!(
        bootstrap::bootstrap(config, &source).await,
        Err(BootstrapError::StorageUnavailable)
    ));
}

#[tokio::test]
async fn shutdown_wakes_all_waiters_and_preexisting_requests_and_drains() {
    let shutdown = Shutdown::default();
    let a = shutdown.clone();
    let b = shutdown.clone();
    let first = tokio::spawn(async move { a.cancelled().await });
    let second = tokio::spawn(async move { b.cancelled().await });
    assert!(shutdown.drain(async {}).await);
    assert!(shutdown.is_requested());
    tokio::time::timeout(std::time::Duration::from_secs(1), async {
        first.await.unwrap();
        second.await.unwrap();
        shutdown.cancelled().await;
    })
    .await
    .unwrap();
}

#[test]
fn sigterm_is_received_in_an_isolated_child() {
    let (mut child, mut reader) = spawn_helper(
        "signal_child",
        &[("RUSTUNE_SIGNAL_TEST_CHILD", std::ffi::OsStr::new("1"))],
        "signal-listener-ready",
    );
    let sent = std::process::Command::new("/bin/kill")
        .args(["-TERM", &child.id().to_string()])
        .status()
        .unwrap();
    assert!(sent.success());
    // 終了までpipeを読み続け、補助プロセスがstdoutで詰まらないようにする。
    std::io::copy(&mut reader, &mut std::io::sink()).unwrap();
    assert!(child.wait().unwrap().success());
}
#[tokio::test]
#[ignore = "helper runs only in an isolated subprocess"]
async fn signal_child() {
    if std::env::var("RUSTUNE_SIGNAL_TEST_CHILD").as_deref() != Ok("1") {
        return;
    }
    // 準備完了を報告する前にハンドラを登録するため、親のSIGTERMが
    // 既定動作（プロセス終了）に当たることはない。
    let signal = bootstrap::ShutdownSignal::install().unwrap();
    println!("signal-listener-ready");
    tokio::time::timeout(std::time::Duration::from_secs(5), signal.recv())
        .await
        .unwrap()
        .unwrap();
}

/// このテストバイナリでignored指定の補助テストを1つ起動し、目印の出力を待つ。
fn spawn_helper(
    helper: &str,
    envs: &[(&str, &std::ffi::OsStr)],
    marker: &str,
) -> (
    std::process::Child,
    std::io::BufReader<std::process::ChildStdout>,
) {
    use std::io::BufRead;
    let mut command = std::process::Command::new(std::env::current_exe().unwrap());
    command
        .args(["--exact", helper, "--ignored", "--nocapture"])
        .stdout(std::process::Stdio::piped());
    for (key, value) in envs {
        command.env(key, value);
    }
    let mut child = command.spawn().unwrap();
    let mut reader = std::io::BufReader::new(child.stdout.take().unwrap());
    loop {
        let mut line = String::new();
        assert!(
            reader.read_line(&mut line).unwrap() > 0,
            "{helper} exited before ready"
        );
        if line.contains(marker) {
            break;
        }
    }
    (child, reader)
}

#[test]
fn second_process_is_rejected_and_crash_releases_lock() {
    let dir = tempfile::tempdir().unwrap();
    let (mut child, _reader) = spawn_helper(
        "lock_holder_child",
        &[("RUSTUNE_LOCK_TEST_DIR", dir.path().as_os_str())],
        "lock-held",
    );
    assert!(matches!(
        ProcessLock::acquire(dir.path()),
        Err(BootstrapError::AlreadyRunning)
    ));
    // 異常終了：プロセスとともにOSがadvisoryロックを解放する。
    child.kill().unwrap();
    child.wait().unwrap();
    let lock = ProcessLock::acquire(dir.path()).unwrap();
    drop(lock);
    assert!(dir.path().join("rustune.lock").exists());
}
#[test]
#[ignore = "helper runs only in an isolated subprocess"]
fn lock_holder_child() {
    let Some(dir) = std::env::var_os("RUSTUNE_LOCK_TEST_DIR") else {
        return;
    };
    let _lock = ProcessLock::acquire(std::path::Path::new(&dir)).unwrap();
    println!("lock-held");
    // 親がこのプロセスを終了させる。待ち時間は親が異常な場合の上限にすぎない。
    std::thread::sleep(std::time::Duration::from_secs(30));
}

#[tokio::test]
async fn drain_timeout_reports_unfinished_work() {
    let shutdown = Shutdown::default();
    let observer = shutdown.clone();
    let finished = shutdown
        .drain_within(std::time::Duration::from_millis(20), async move {
            observer.cancelled().await;
            std::future::pending::<()>().await;
        })
        .await;
    assert!(!finished);
    assert!(shutdown.is_requested());
    assert_eq!(
        bootstrap::SHUTDOWN_DRAIN_TIMEOUT,
        std::time::Duration::from_secs(10)
    );
}

#[tokio::test]
async fn concurrent_settings_updates_have_one_winner() {
    let (_dir, r) = repo().await;
    let base = r.settings().await.unwrap();
    let mut a = base.clone();
    a.interval_seconds = 120;
    let mut b = base.clone();
    b.interval_seconds = 300;
    let (ra, rb) = tokio::join!(r.update_settings(0, &a), r.update_settings(0, &b));
    assert_eq!(usize::from(ra.is_ok()) + usize::from(rb.is_ok()), 1);
    assert!(matches!(ra, Err(DbError::Conflict)) || matches!(rb, Err(DbError::Conflict)));
    let stored = r.settings().await.unwrap();
    assert_eq!(stored.revision, 1);
    assert!([120, 300].contains(&stored.interval_seconds));
    assert_eq!(r.library_revision().await.unwrap(), 1);
    // 範囲外の値はSQLの前に拒否し、既定値へ置き換えない。
    let mut invalid = stored.clone();
    invalid.interval_seconds = 29;
    assert!(matches!(
        r.update_settings(1, &invalid).await,
        Err(DbError::InvalidInput)
    ));
    invalid.interval_seconds = 60;
    invalid.reserve_free_bytes = 1;
    assert!(matches!(
        r.update_settings(1, &invalid).await,
        Err(DbError::InvalidInput)
    ));
    assert_eq!(r.settings().await.unwrap(), stored);
}
