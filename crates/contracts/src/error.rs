//! 失敗の伝え方の契約：HTTPのエラー応答と、取り込み結果の理由に使うエラーcode。
//!
//! # 処理フロー
//! ```mermaid
//! sequenceDiagram
//!     participant ui as UI（web）
//!     participant api as server（HTTP）
//!     participant svc as server（各処理）
//!     ui->>api: リクエスト
//!     api->>svc: 処理を依頼
//!     svc-->>api: 失敗（内部のエラー型）
//!     api->>api: ErrorCodeへ変換し、ErrorCode::responseでApiErrorを作る
//!     api-->>ui: HTTP 4xx/5xx と ApiError（JSON）
//!     ui->>ui: codeで表示を決める。知らないcodeならmessageをそのまま表示
//! ```
//! 内部のエラー（OSのエラー、パーサーのエラーなど）の文言は外へ出さず、必ず`ErrorCode`の定型文に置き換える。
//!
use serde::{Deserialize, Serialize};
/// 失敗時にserverが返すJSONのbody。全APIで共通の形。
/// 将来追加される未知のcodeも、UIの汎用フォールバック表示のために読み取れる。
///
/// - `code`：機械向けの識別子（[`ErrorCode::as_str`]の値）。UIはこれで表示や次の操作を決める。
///   `ErrorCode`ではなく`String`で持つのは、UIより新しいserverが知らないcodeを返しても、deserializeに失敗させないため。
/// - `message`：利用者向けの日本語の定型文（[`ErrorCode::message`]）。OSやパーサーのエラー文字列は入れない。
/// - `retryable`：同じ操作をもう一度試す意味があるか。例：一時的な読み取り失敗は`true`、未対応の形式は`false`。
/// - `request_id`：serverのログと突き合わせるためのID。利用者が問い合わせるときの手がかりにする。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiError {
    pub code: String,
    pub message: String,
    pub retryable: bool,
    pub request_id: String,
}
/// serverが返すエラーcodeの一覧。HTTPのエラー応答だけでなく、取り込み結果の理由としても使う。
///
/// # 使われる場所
/// - **HTTPのエラー応答**：serverが[`ErrorCode::response`]で[`ApiError`]を作って返す。
///   HTTPステータスとの対応はMODULE_DESIGN §2の表（例：`invalid_input`は400、`revision_conflict`は409）。
/// - **取り込みの理由**：[`crate::imports::ImportItem::reason`]と、
///   [`crate::imports::ImportJob::reason_counts`]のキーに`as_str()`の値が入る。
///   例：購入曲と確認できないファイルは`skipped`で、理由は`purchase_unverified`。
/// - **画像の警告**：`artwork_*`は、音源の取り込みは成功したが画像を使えなかったことを表す。
///
/// # 変更するとき
/// codeの文字列はAPIの一部なので、一度公開した値は変えない。追加するときはMODULE_DESIGN §2の表も更新する。
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
    AnalysisLimitExceeded,
    FileTooLarge,
    PurchaseUnverified,
    FileUnstable,
    CorruptAudio,
    Interrupted,
    IoFailure,
    UpstreamUnavailable,
    UpstreamRateLimited,
    IdentityConflict,
    ArtworkTooLarge,
    ArtworkUnsupported,
    ArtworkInvalid,
}
impl ErrorCode {
    /// JSONに出すcodeの文字列（snake_case）。
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
            Self::AnalysisLimitExceeded => "analysis_limit_exceeded",
            Self::FileTooLarge => "file_too_large",
            Self::PurchaseUnverified => "purchase_unverified",
            Self::FileUnstable => "file_unstable",
            Self::CorruptAudio => "corrupt_audio",
            Self::Interrupted => "interrupted",
            Self::IoFailure => "io_failure",
            Self::UpstreamUnavailable => "upstream_unavailable",
            Self::UpstreamRateLimited => "upstream_rate_limited",
            Self::IdentityConflict => "identity_conflict",
            Self::ArtworkTooLarge => "artwork_too_large",
            Self::ArtworkUnsupported => "artwork_unsupported",
            Self::ArtworkInvalid => "artwork_invalid",
        }
    }
    /// 利用者向けの日本語の定型文。UIはこの文をそのまま表示できる。
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
            Self::AnalysisLimitExceeded => "音源の構造が解析できる上限を超えています。",
            Self::FileTooLarge => "音源ファイルが取り込める大きさを超えています。",
            Self::PurchaseUnverified => "購入由来の情報を確認できません。",
            Self::FileUnstable => "ファイルの書き込み完了を待っています。",
            Self::CorruptAudio => "音源の構造を読み取れません。",
            Self::Interrupted => "処理が中断されました。",
            Self::IoFailure => "ファイル処理に失敗しました。",
            Self::UpstreamUnavailable => "補完サービスに接続できません。",
            Self::UpstreamRateLimited => "しばらく待ってから補完を再実行してください。",
            Self::IdentityConflict => "楽曲の識別情報が競合しています。",
            Self::ArtworkTooLarge => "ジャケット画像が大きすぎるため、既定の画像を表示します。",
            Self::ArtworkUnsupported => "対応していない画像形式のため、既定の画像を表示します。",
            Self::ArtworkInvalid => "ジャケット画像を読み取れないため、既定の画像を表示します。",
        }
    }
    /// HTTPのエラー応答のbodyを作る。`retryable`と`request_id`はserverが状況に応じて渡す。
    pub fn response(self, retryable: bool, request_id: String) -> ApiError {
        ApiError {
            code: self.as_str().to_owned(),
            message: self.message().to_owned(),
            retryable,
            request_id,
        }
    }
}
