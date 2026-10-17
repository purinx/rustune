use super::*;
use std::io::Read;
use std::os::unix::ffi::OsStringExt;
use std::os::unix::fs::symlink;

#[test]
fn enumerates_only_candidates_without_lossy_names_or_mutation() {
    let dir = tempfile::tempdir().unwrap();
    let name = std::ffi::OsString::from("日本語.M4A");
    let path = dir.path().join(&name);
    std::fs::write(&path, b"source").unwrap();
    std::fs::write(dir.path().join("other.mp3"), b"x").unwrap();
    let before = std::fs::metadata(&path).unwrap();
    let root = SourceRoot::open(dir.path()).unwrap();
    let mut candidates = vec![];
    let report = root
        .enumerate(|candidate| candidates.push(candidate))
        .unwrap();
    assert_eq!(report.candidates, 1);
    assert!(report.complete);
    assert_eq!(report.ignored_extensions.get("mp3"), Some(&1));
    assert_eq!(candidates[0].relative_path, PathBuf::from(name));
    let mut source = root.open_candidate(&candidates[0]).unwrap();
    let mut content = String::new();
    source.reader().read_to_string(&mut content).unwrap();
    assert_eq!(content, "source");
    source.verify_unchanged().unwrap();
    let after = std::fs::metadata(path).unwrap();
    assert_eq!(before.len(), after.len());
    assert_eq!(before.modified().unwrap(), after.modified().unwrap());
}

#[test]
fn non_utf8_components_are_preserved_without_lossy_conversion() {
    let name = std::ffi::OsString::from_vec(vec![0xff, b'.', b'm', b'4', b'a']);
    let path = PathBuf::from(name.clone());
    assert_eq!(checked_components(&path).unwrap()[0], name);
}

// APFSは不正なUTF-8のファイル名を作成時に拒否するため、実際の非UTF-8 entryは
// そのような名前を扱えるLinuxでのみ検証する。macOSでは成分の無損失処理を検証する。
#[cfg(target_os = "linux")]
#[test]
fn enumerates_actual_non_utf8_file() {
    let dir = tempfile::tempdir().unwrap();
    let name = std::ffi::OsString::from_vec(vec![0xff, b'.', b'm', b'4', b'a']);
    std::fs::write(dir.path().join(&name), b"source").unwrap();
    let root = SourceRoot::open(dir.path()).unwrap();
    let mut names = vec![];
    root.enumerate(|c| names.push(c.relative_path)).unwrap();
    assert_eq!(names, vec![PathBuf::from(name)]);
}

#[test]
fn refuses_symlinks_special_files_and_traversal() {
    let dir = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    std::fs::write(outside.path().join("secret.m4a"), b"outside").unwrap();
    symlink(outside.path(), dir.path().join("link")).unwrap();
    symlink(
        outside.path().join("secret.m4a"),
        dir.path().join("file.m4a"),
    )
    .unwrap();
    assert!(
        std::process::Command::new("mkfifo")
            .arg(dir.path().join("pipe.m4a"))
            .status()
            .unwrap()
            .success()
    );
    let _socket = std::os::unix::net::UnixListener::bind(dir.path().join("socket.m4a")).unwrap();
    let root = SourceRoot::open(dir.path()).unwrap();
    assert!(SourceRoot::open(&dir.path().join("link")).is_err());
    for path in [
        "../secret.m4a",
        "/etc/passwd",
        "file.m4a",
        "link/secret.m4a",
        "pipe.m4a",
        "socket.m4a",
    ] {
        assert!(root.open_readonly(Path::new(path)).is_err());
    }
    let report = root.enumerate(|_| panic!("unexpected candidate")).unwrap();
    assert!(!report.complete);
    assert!(report.warnings >= 4);
}

#[test]
fn rejects_swapped_parent_and_changed_file() {
    let dir = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("album")).unwrap();
    std::fs::write(dir.path().join("album/a.m4a"), b"original").unwrap();
    std::fs::write(outside.path().join("a.m4a"), b"outside").unwrap();
    let root = SourceRoot::open(dir.path()).unwrap();
    let mut candidates = vec![];
    root.enumerate(|c| candidates.push(c)).unwrap();
    let pinned = root.open_candidate(&candidates[0]).unwrap();
    std::fs::rename(dir.path().join("album"), dir.path().join("old")).unwrap();
    symlink(outside.path(), dir.path().join("album")).unwrap();
    assert!(root.open_candidate(&candidates[0]).is_err());
    // 既に開いたハンドルは元のinodeに結び付いたまま。
    pinned.verify_unchanged().unwrap();
    std::fs::write(dir.path().join("old/a.m4a"), b"modified").unwrap();
    assert_eq!(pinned.verify_unchanged(), Err(SourceError::Changed));
}

#[test]
fn classifies_open_failures_into_contract_codes() {
    use rustune_contracts::error::ErrorCode;
    let dir = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    std::fs::write(outside.path().join("secret.m4a"), b"outside").unwrap();
    symlink(outside.path(), dir.path().join("link")).unwrap();
    symlink(
        outside.path().join("secret.m4a"),
        dir.path().join("file.m4a"),
    )
    .unwrap();
    std::fs::create_dir(dir.path().join("album.m4a")).unwrap();
    let root = SourceRoot::open(dir.path()).unwrap();
    let open = |path: &str| root.open_readonly(Path::new(path)).err().unwrap();
    assert_eq!(open("../secret.m4a"), SourceError::InvalidPath);
    assert_eq!(open("/etc/passwd"), SourceError::InvalidPath);
    assert_eq!(open("a/./b/../c.m4a"), SourceError::InvalidPath);
    assert_eq!(open("file.m4a"), SourceError::NotRegular);
    assert_eq!(open("link/secret.m4a"), SourceError::NotRegular);
    assert_eq!(open("album.m4a"), SourceError::NotRegular);
    assert_eq!(open("missing.m4a"), SourceError::NotFound);
    assert_eq!(
        SourceError::InvalidPath.error_code(),
        ErrorCode::InvalidInput
    );
    assert_eq!(
        SourceError::NotRegular.error_code(),
        ErrorCode::FileUnstable
    );
    assert_eq!(
        SourceError::PermissionDenied.error_code(),
        ErrorCode::SourcePermissionDenied
    );
    // root自身がシンボリックリンク・ファイル・存在しない場合は設定rootのエラー。
    for path in [
        dir.path().join("link"),
        dir.path().join("missing"),
        outside.path().join("secret.m4a"),
    ] {
        let error = SourceRoot::open(&path).err().unwrap();
        assert_eq!(error, SourceError::RootUnavailable);
        assert_eq!(error.error_code(), ErrorCode::InvalidSourceRoot);
    }
    assert_eq!(
        SourceRoot::open(Path::new("relative")).err(),
        Some(SourceError::InvalidPath)
    );
}

#[test]
fn unreadable_subdirectory_is_a_warning_not_a_complete_scan() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let locked = dir.path().join("locked");
    std::fs::create_dir(&locked).unwrap();
    std::fs::write(locked.join("hidden.m4a"), b"x").unwrap();
    std::fs::write(dir.path().join("visible.m4p"), b"x").unwrap();
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000)).unwrap();
    let root = SourceRoot::open(dir.path()).unwrap();
    let mut names = vec![];
    let report = root.enumerate(|c| names.push(c.relative_path));
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o755)).unwrap();
    let report = report.unwrap();
    assert_eq!(names, vec![PathBuf::from("visible.m4p")]);
    assert!(!report.complete);
    assert_eq!(report.warnings, 1);
    assert_eq!(
        root.open_readonly(Path::new("locked/hidden.m4a")).err(),
        None,
        "permissions restored"
    );
}

#[test]
fn intermediate_directory_swapped_to_symlink_during_scan_is_not_followed() {
    let dir = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("a/b")).unwrap();
    std::fs::write(dir.path().join("a/b/x.m4a"), b"inside").unwrap();
    std::fs::create_dir(outside.path().join("b")).unwrap();
    std::fs::write(outside.path().join("b/x.m4a"), b"outside").unwrap();
    let root = SourceRoot::open(dir.path()).unwrap();
    let mut candidates = vec![];
    root.enumerate(|c| candidates.push(c)).unwrap();
    assert_eq!(candidates.len(), 1);
    // 中間階層aを外部へのリンクに置き換えると、同じ相対パスでも辿らない。
    std::fs::rename(dir.path().join("a"), dir.path().join("moved")).unwrap();
    symlink(outside.path(), dir.path().join("a")).unwrap();
    assert_eq!(
        root.open_candidate(&candidates[0]).err(),
        Some(SourceError::NotRegular)
    );
    let mut after = vec![];
    let report = root.enumerate(|c| after.push(c.relative_path)).unwrap();
    assert_eq!(after, vec![PathBuf::from("moved/b/x.m4a")]);
    assert_eq!(report.warnings, 1);
}

#[test]
fn verify_entry_detects_replacement_after_open() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("a.m4a");
    std::fs::write(&path, b"original").unwrap();
    let root = SourceRoot::open(dir.path()).unwrap();
    let opened = root.open_readonly(Path::new("a.m4a")).unwrap();
    root.verify_entry(Path::new("a.m4a"), &opened).unwrap();
    // 同じ内容でも別inodeへの差し替えは別ファイルとして扱う。
    std::fs::write(dir.path().join("new"), b"original").unwrap();
    std::fs::rename(dir.path().join("new"), &path).unwrap();
    assert_eq!(
        root.verify_entry(Path::new("a.m4a"), &opened),
        Err(SourceError::Changed)
    );
}
