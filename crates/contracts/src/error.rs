use serde::{Deserialize, Serialize};
/// 将来追加される未知のcodeも、UIの汎用フォールバック表示のために読み取れる。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiError {
    pub code: String,
    pub message: String,
    pub retryable: bool,
    pub request_id: String,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorCode {
    InvalidInput,
    InvalidSourceRoot,
    NotFound,
    MediaMissing,
    RevisionConflict,
    ImportBusy,
    SourcePermissionDenied,
    StorageFull,
    UnsupportedCodec,
    PurchaseUnverified,
    FileUnstable,
    CorruptAudio,
    Interrupted,
    IoFailure,
    UpstreamUnavailable,
    UpstreamRateLimited,
    IdentityConflict,
}
impl ErrorCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::InvalidInput => "invalid_input",
            Self::InvalidSourceRoot => "invalid_source_root",
            Self::NotFound => "not_found",
            Self::MediaMissing => "media_missing",
            Self::RevisionConflict => "revision_conflict",
            Self::ImportBusy => "import_busy",
            Self::SourcePermissionDenied => "source_permission_denied",
            Self::StorageFull => "storage_full",
            Self::UnsupportedCodec => "unsupported_codec",
            Self::PurchaseUnverified => "purchase_unverified",
            Self::FileUnstable => "file_unstable",
            Self::CorruptAudio => "corrupt_audio",
            Self::Interrupted => "interrupted",
            Self::IoFailure => "io_failure",
            Self::UpstreamUnavailable => "upstream_unavailable",
            Self::UpstreamRateLimited => "upstream_rate_limited",
            Self::IdentityConflict => "identity_conflict",
        }
    }
    pub fn message(self) -> &'static str {
        match self {
            Self::InvalidInput => "入力内容を確認してください。",
            Self::InvalidSourceRoot => "取り込み元フォルダを確認してください。",
            Self::NotFound => "対象が見つかりません。",
            Self::MediaMissing => "保存音源が見つかりません。",
            Self::RevisionConflict => "情報が更新されています。読み直してください。",
            Self::ImportBusy => "取り込み処理を実行中です。",
            Self::SourcePermissionDenied => "取り込み元を読み取れません。",
            Self::StorageFull => "保存先の空き容量が不足しています。",
            Self::UnsupportedCodec => "対応していない音源形式です。",
            Self::PurchaseUnverified => "購入由来の情報を確認できません。",
            Self::FileUnstable => "ファイルの書き込み完了を待っています。",
            Self::CorruptAudio => "音源の構造を読み取れません。",
            Self::Interrupted => "処理が中断されました。",
            Self::IoFailure => "ファイル処理に失敗しました。",
            Self::UpstreamUnavailable => "補完サービスに接続できません。",
            Self::UpstreamRateLimited => "しばらく待ってから補完を再実行してください。",
            Self::IdentityConflict => "楽曲の識別情報が競合しています。",
        }
    }
    pub fn response(self, retryable: bool, request_id: String) -> ApiError {
        ApiError {
            code: self.as_str().to_owned(),
            message: self.message().to_owned(),
            retryable,
            request_id,
        }
    }
}
