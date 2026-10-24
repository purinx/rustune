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
    UnverifiableRoots,
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
            Self::UnverifiableRoots => "source and storage separation cannot be verified",
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
/// リリース配置での既定Web資産ディレクトリ名。実行ファイルと同じディレクトリに置く。
pub const RELEASE_WEB_DIR: &str = "web";

/// 実行中のバイナリの位置を基準に設定を読み込む。既定の`web_dist`は
/// `<実行ファイルのディレクトリ>/web`（DESIGN_DOCのリリース配置）となる。
pub fn load_config(text: &str, home: &Path) -> Result<RuntimeConfig, ConfigError> {
    let exe = std::env::current_exe()
        .and_then(std::fs::canonicalize)
        .map_err(|_| ConfigError::Io)?;
    let release_dir = exe.parent().ok_or(ConfigError::Io)?;
    load_config_with_release_dir(text, home, release_dir)
}
/// `release_dir`はバイナリを置いたリリースディレクトリ（絶対パス）。テストや
/// ランチャーが実行ファイルの位置に依存せず既定値を決めるために使う。
pub fn load_config_with_release_dir(
    text: &str,
    home: &Path,
    release_dir: &Path,
) -> Result<RuntimeConfig, ConfigError> {
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
    // 明示指定も既定値も絶対パスに限る。起動時の作業ディレクトリに依存させない。
    let web_dist = absolute_clean(
        &parsed
            .web_dist
            .unwrap_or_else(|| release_dir.join(RELEASE_WEB_DIR)),
    )?;
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
/// 解決できる最も深い祖先を実パスへ解決し、残りの成分を後ろに足したパス。
struct ResolvedPath {
    path: PathBuf,
    /// 権限不足で解決できなかった成分が残っている。そこに別名（symlink）が
    /// 隠れている可能性があるため、分離を確認済みとはみなさない。
    unverified_tail: bool,
}
/// 比較の前に存在する祖先を実パスへ解決する。取り込み元ディレクトリが未作成でも、
/// 途中の祖先が読めなくても、解決できる祖先までのシンボリックリンク別名を保つ。
/// リンク先が未作成のsymlinkも、リンク先のパスへ置き換えて比較する（[`LinkResolver`]）。
fn resolve_deepest_ancestor(path: &Path) -> Result<ResolvedPath, ConfigError> {
    let mut resolver = LinkResolver::default();
    let path = resolver.resolve(&absolute_clean(path)?)?;
    Ok(ResolvedPath {
        path,
        unverified_tail: resolver.unverified,
    })
}

/// symlinkのたどりすぎ（循環リンク）とみなす回数。Linuxのカーネルの上限（40）に合わせる。
const MAX_LINK_FOLLOWS: usize = 40;

/// パスを実パスへ解決する。`canonicalize`は、リンク先が未作成のsymlinkにも`NotFound`を返すため、
/// そのままでは未解決の別名を「別の場所」と誤判定する。そこで`symlink_metadata`と`read_link`で
/// 未作成のリンク先を自分でたどる。
#[derive(Default)]
struct LinkResolver {
    /// これまでにたどったsymlinkの数。
    follows: usize,
    /// 権限不足で解決できなかった成分があった（[`ResolvedPath::unverified_tail`]）。
    unverified: bool,
}
impl LinkResolver {
    /// 解決できる最も深い祖先を実パスへ解決し、残りの成分を後ろに足したパスを返す。
    fn resolve(&mut self, path: &Path) -> Result<PathBuf, ConfigError> {
        let mut ancestor = path.to_path_buf();
        let mut tail = Vec::new();
        loop {
            match std::fs::canonicalize(&ancestor) {
                Ok(resolved) => return Ok(push_tail(resolved, &tail)),
                Err(e) => match e.kind() {
                    std::io::ErrorKind::NotFound | std::io::ErrorKind::NotADirectory => {
                        // リンク先が未作成のsymlinkなら、リンク先を解決して残りの成分を足す。
                        if let Some(target) = self.dangling_link_target(&ancestor)? {
                            let resolved = self.resolve(&target)?;
                            return Ok(push_tail(resolved, &tail));
                        }
                        // 存在しない成分は別名になり得ないため、確定した残り成分として扱う。
                    }
                    // 読めない祖先より下は実体を確認できない。HTTPは止めず、状態は
                    // inspect_source_rootで別途報告する。取り込み元はここでは開かない。
                    std::io::ErrorKind::PermissionDenied => self.unverified = true,
                    _ => return Err(ConfigError::Io),
                },
            }
            tail.push(
                ancestor
                    .file_name()
                    .ok_or(ConfigError::InvalidPath)?
                    .to_os_string(),
            );
            ancestor = ancestor
                .parent()
                .ok_or(ConfigError::InvalidPath)?
                .to_path_buf();
        }
    }

    /// `link`がsymlinkなら、そのリンク先の絶対パスを返す。symlinkでなければ`None`。
    /// 相対リンクは、リンクを置いたディレクトリ（を実パスへ解決したもの）を基準にする。
    fn dangling_link_target(&mut self, link: &Path) -> Result<Option<PathBuf>, ConfigError> {
        let Ok(meta) = std::fs::symlink_metadata(link) else {
            return Ok(None);
        };
        if !meta.file_type().is_symlink() {
            return Ok(None);
        }
        self.follows += 1;
        if self.follows > MAX_LINK_FOLLOWS {
            return Err(ConfigError::Io);
        }
        let target = std::fs::read_link(link).map_err(|_| ConfigError::Io)?;
        let base = if target.is_absolute() {
            PathBuf::from("/")
        } else {
            self.resolve(link.parent().ok_or(ConfigError::InvalidPath)?)?
        };
        self.join(base, &target).map(Some)
    }

    /// `base`に`target`の成分を順に足す。`..`は、その時点までを実パスへ解決してから1つ戻る。
    /// 文字列の上で`..`を畳むと、途中のsymlinkを無視した誤ったパスになるため。
    fn join(&mut self, base: PathBuf, target: &Path) -> Result<PathBuf, ConfigError> {
        let mut path = base;
        for component in target.components() {
            match component {
                Component::RootDir | Component::Prefix(_) => path = PathBuf::from("/"),
                Component::CurDir => {}
                Component::Normal(part) => path.push(part),
                Component::ParentDir => {
                    path = self.resolve(&path)?;
                    path.pop();
                }
            }
        }
        Ok(path)
    }
}

/// 解決済みのパスに、解決できなかった残りの成分（逆順に積んだもの）を足す。
fn push_tail(mut path: PathBuf, tail: &[std::ffi::OsString]) -> PathBuf {
    for part in tail.iter().rev() {
        path.push(part);
    }
    path
}
/// 取り込み元と保存先の位置関係。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RootRelation {
    /// 両方を最後まで解決でき、包含関係がない。
    Separate,
    /// 解決後のパスで一方が他方を含む（読めない部分を含む場合も安全側で重なりとみなす）。
    Overlapping,
    /// 読めない祖先があり、その下の別名を確認できないため分離を確認できない。
    Unverifiable,
}
pub fn root_relation(source: &Path, storage: &Path) -> Result<RootRelation, ConfigError> {
    let source = resolve_deepest_ancestor(source)?;
    let storage = resolve_deepest_ancestor(storage)?;
    if source.path.starts_with(&storage.path) || storage.path.starts_with(&source.path) {
        Ok(RootRelation::Overlapping)
    } else if source.unverified_tail || storage.unverified_tail {
        Ok(RootRelation::Unverifiable)
    } else {
        Ok(RootRelation::Separate)
    }
}
/// 設定変更など、分離を確認できた場合だけ受け付ける厳格な検証。
pub fn validate_separate_roots(source: &Path, storage: &Path) -> Result<(), ConfigError> {
    match root_relation(source, storage)? {
        RootRelation::Separate => Ok(()),
        RootRelation::Overlapping => Err(ConfigError::OverlappingRoots),
        RootRelation::Unverifiable => Err(ConfigError::UnverifiableRoots),
    }
}
/// 起動時の検証。確認済みの重なりだけを起動失敗とする。確認できない取り込み元は
/// 読めないため取り込みも行えず、状態（PermissionDenied等）として報告して起動を続ける。
pub fn reject_overlapping_roots(source: &Path, storage: &Path) -> Result<(), ConfigError> {
    match root_relation(source, storage)? {
        RootRelation::Overlapping => Err(ConfigError::OverlappingRoots),
        RootRelation::Separate | RootRelation::Unverifiable => Ok(()),
    }
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
