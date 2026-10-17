//! 実行時TOMLは運用設定を所有する。取り込み関連の設定はSQLiteだけに保存する。
use serde::Deserialize;
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::{Component, Path, PathBuf},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConfigError {
    InvalidToml,
    InvalidPort,
    NonLoopback,
    InvalidPath,
    OverlappingRoots,
    Io,
}
impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::InvalidToml => "invalid runtime configuration",
            Self::InvalidPort => "port must be between 1 and 65535",
            Self::NonLoopback => "server must bind to loopback",
            Self::InvalidPath => "invalid absolute directory path",
            Self::OverlappingRoots => "source and storage directories overlap",
            Self::Io => "configuration path cannot be inspected",
        })
    }
}
impl std::error::Error for ConfigError {}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TomlConfig {
    bind: Option<IpAddr>,
    port: Option<u16>,
    data_dir: Option<PathBuf>,
    web_dist: Option<PathBuf>,
    log_filter: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeConfig {
    pub address: SocketAddr,
    pub data_dir: PathBuf,
    pub web_dist: PathBuf,
    pub log_filter: String,
}
pub fn load_config(text: &str, home: &Path) -> Result<RuntimeConfig, ConfigError> {
    let parsed: TomlConfig = toml::from_str(text).map_err(|_| ConfigError::InvalidToml)?;
    let bind = parsed.bind.unwrap_or(IpAddr::V4(Ipv4Addr::LOCALHOST));
    if !bind.is_loopback() {
        return Err(ConfigError::NonLoopback);
    }
    let port = parsed.port.unwrap_or(3867);
    if port == 0 {
        return Err(ConfigError::InvalidPort);
    }
    let data_dir = absolute_clean(
        &parsed
            .data_dir
            .unwrap_or_else(|| home.join("Library/Application Support/Rustune")),
    )?;
    let web_dist = parsed
        .web_dist
        .unwrap_or_else(|| PathBuf::from("crates/web/dist"));
    if web_dist.as_os_str().is_empty() {
        return Err(ConfigError::InvalidPath);
    }
    let log_filter = parsed
        .log_filter
        .unwrap_or_else(|| "rustune_server=info".into());
    if log_filter.trim().is_empty() {
        return Err(ConfigError::InvalidToml);
    }
    tracing_subscriber::EnvFilter::try_new(&log_filter).map_err(|_| ConfigError::InvalidToml)?;
    Ok(RuntimeConfig {
        address: SocketAddr::new(bind, port),
        data_dir,
        web_dist,
        log_filter,
    })
}
pub fn default_source_root(home: &Path) -> PathBuf {
    home.join("Music/Music/Media.localized/Music")
}
fn absolute_clean(path: &Path) -> Result<PathBuf, ConfigError> {
    if !path.is_absolute()
        || path.components().any(|c| matches!(c, Component::ParentDir))
        || path.parent().is_none()
    {
        return Err(ConfigError::InvalidPath);
    }
    Ok(path.components().collect())
}
/// 比較の前に存在する祖先を実パスへ解決する。取り込み元ディレクトリが未作成でも
/// シンボリックリンク経由の別名を検出できる。
fn resolved_or_missing(path: &Path) -> Result<PathBuf, ConfigError> {
    let clean = absolute_clean(path)?;
    let mut ancestor = clean.as_path();
    let mut missing = Vec::new();
    loop {
        match std::fs::canonicalize(ancestor) {
            Ok(mut resolved) => {
                for part in missing.iter().rev() {
                    resolved.push(part);
                }
                return Ok(resolved);
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                missing.push(
                    ancestor
                        .file_name()
                        .ok_or(ConfigError::InvalidPath)?
                        .to_os_string(),
                );
                ancestor = ancestor.parent().ok_or(ConfigError::InvalidPath)?;
            }
            // 取り込み元の不在・読み取り不可でHTTPを止めない。状態は別途報告し、
            // ここでは取り込みのために取り込み元を開かない。
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => return Ok(clean),
            Err(_) => return Err(ConfigError::Io),
        }
    }
}
pub fn validate_separate_roots(source: &Path, storage: &Path) -> Result<(), ConfigError> {
    let source = resolved_or_missing(source)?;
    let storage = resolved_or_missing(storage)?;
    if source.starts_with(&storage) || storage.starts_with(&source) {
        return Err(ConfigError::OverlappingRoots);
    }
    Ok(())
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceStatus {
    Available,
    Missing,
    PermissionDenied,
    NotDirectory,
    RejectedSymlink,
    Unavailable,
}
pub fn inspect_source_root(path: &Path) -> SourceStatus {
    match std::fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_symlink() => SourceStatus::RejectedSymlink,
        Ok(meta) if !meta.is_dir() => SourceStatus::NotDirectory,
        Ok(_) => match std::fs::read_dir(path) {
            Ok(_) => SourceStatus::Available,
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                SourceStatus::PermissionDenied
            }
            Err(_) => SourceStatus::Unavailable,
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => SourceStatus::Missing,
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
            SourceStatus::PermissionDenied
        }
        Err(_) => SourceStatus::Unavailable,
    }
}
