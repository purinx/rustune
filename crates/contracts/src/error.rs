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
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiError {
    /// 機械向けの識別子（[`ErrorCode::as_str`]の値）。UIはこれで表示や次の操作を決める。
    /// `ErrorCode`ではなく`String`で持つのは、UIより新しいserverが知らないcodeを返しても、deserializeに失敗させないため。
    pub code: String,
    /// 利用者向けの日本語の定型文（[`ErrorCode::message`]）。OSやパーサーのエラー文字列は入れない。
    pub message: String,
    /// 同じ操作をもう一度試す意味があるか。例：一時的な読み取り失敗は`true`、未対応の形式は`false`。
    pub retryable: bool,
    /// serverのログと突き合わせるためのID。
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
    /// 入力が不正（`validate`の失敗など）。HTTP 400。
    InvalidInput,
    /// 取り込み元フォルダが存在しない・フォルダではない・保存先と重なっている。HTTP 400。
    InvalidSourceRoot,
    /// 指定したIDの対象がない。HTTP 404。
    NotFound,
    /// 曲はあるが、保存した音源ファイルが見つからない。HTTP 404。
    MediaMissing,
    /// `expected_revision`が現在のrevisionと違う（他で先に更新された）。HTTP 409。
    RevisionConflict,
    /// 取り込みジョブの実行中なのでできない操作。HTTP 409。
    ImportBusy,
    /// 取り込み元を読む権限がない。取り込みの理由、または必要ならAPIエラー。
    SourcePermissionDenied,
    /// 保存先の空き容量が設定の下限を下回る。取り込みの理由、または必要ならAPIエラー。
    StorageFull,
    /// 対応していない音源形式（AAC以外など）。取り込みの理由（対象外）。
    UnsupportedCodec,
    /// 音源の構造が解析の上限（箱の数・深さ・タグの大きさなど）を超えた。取り込みの理由（対象外）。
    AnalysisLimitExceeded,
    /// ファイルが設定の`max_file_bytes`を超えた。取り込みの理由（対象外）。
    FileTooLarge,
    /// 購入した曲だと確認できない。取り込みの理由（対象外）。
    PurchaseUnverified,
    /// コピー中にファイルが変わった、または書き込み途中の可能性がある。次の走査で再評価する。
    FileUnstable,
    /// 音源の構造が壊れていて読めない。取り込みの理由（失敗）。
    CorruptAudio,
    /// 処理が中断された（終了・キャンセル）。復旧後に再試行する。
    Interrupted,
    /// ファイルの読み書きに失敗した。
    IoFailure,
    /// 補完サービス（iTunes Search API）に接続できない。HTTP 502。
    UpstreamUnavailable,
    /// 補完サービスの利用回数の制限にかかった。HTTP 503。
    UpstreamRateLimited,
    /// 識別情報（ストアIDや音声digest）が、それぞれ別の既存の曲に一致した。自動で統合せず、この理由で止める。
    IdentityConflict,
    /// ジャケット画像が大きすぎる（10 MiB・16メガピクセルを超える）。画像の警告で、音源は取り込む。
    ArtworkTooLarge,
    /// ジャケット画像がJPEG・PNG以外。画像の警告で、音源は取り込む。
    ArtworkUnsupported,
    /// ジャケット画像が壊れていて読めない。画像の警告で、音源は取り込む。
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
