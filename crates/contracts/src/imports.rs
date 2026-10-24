//! 取り込みジョブの契約：取り込みを始める操作と、その進み具合・結果の表示。
//!
//! # 処理フロー
//! ```mermaid
//! sequenceDiagram
//!     participant ui as UI（取り込み画面）
//!     participant api as server（HTTP）
//!     participant worker as server（取り込み処理）
//!     ui->>api: POST /imports
//!     api->>worker: ジョブを予約（実行中なら既存のジョブ）
//!     api-->>ui: 202 ImportAccepted { job_id }
//!     loop 取り込み中は2秒ごと
//!         ui->>api: GET /imports/:job_id
//!         api-->>ui: ImportJob（phase・件数・理由ごとの件数）
//!     end
//!     ui->>api: GET /imports/:job_id/items
//!     api-->>ui: Page<ImportItem>（ファイルごとの結果と理由）
//! ```
//! 起動時（`startup`）と定期走査（`scheduled`）のジョブも、同じ型で表示する。
//! 任意のファイルパスを受け取って取り込むAPIは作らない。対象は設定した取り込み元フォルダの中だけ。
//!
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use uuid::Uuid;
/// 取り込みを始めたきっかけ。`manual`は画面のボタン、`startup`はserver起動時、`scheduled`は定期走査。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportTrigger {
    Manual,
    Startup,
    Scheduled,
}
/// ジョブ全体の状態。`queued`→`running`→`completed`か`failed`。
/// 個々のファイルが失敗してもジョブは`completed`になり、`failed`の件数で表す。
/// ジョブが`failed`になるのは、取り込み元フォルダ全体を読めない、空き容量が尽きた、など続けられない場合。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Queued,
    Running,
    Completed,
    Failed,
}
/// 実行中の段階。`enumerating`は対象ファイルを数えている途中（`total`はまだ`None`）、
/// `processing`は件数が確定して1件ずつ処理している段階。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportPhase {
    Enumerating,
    Processing,
}
/// ファイル1件の状態。`pending`→`checking`（解析中）→`copying`（保存先へコピー中）→`imported`。
/// 途中で対象外とわかれば`skipped`、処理に失敗すれば`failed`になり、理由は[`ImportItem::reason`]に入る。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemStatus {
    Pending,
    Checking,
    Copying,
    Imported,
    Skipped,
    Failed,
}
/// `GET /api/v1/imports`・`/imports/:id`の応答。取り込みジョブ1回分の進み具合と集計。
///
/// - `total`：対象ファイルの総数。数えている途中（`enumerating`）は`None`。
/// - `processed`：終わった件数で、`added + skipped + failed`と常に一致する。
/// - `reason_counts`：理由ごとの件数。キーは[`crate::error::ErrorCode`]の文字列（例：`purchase_unverified`）。
///   `BTreeMap`なので、キーの順番が毎回同じになる。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportJob {
    pub id: Uuid,
    pub trigger: ImportTrigger,
    pub status: JobStatus,
    pub phase: ImportPhase,
    pub total: Option<u64>,
    pub processed: u64,
    pub added: u64,
    pub skipped: u64,
    pub failed: u64,
    pub reason_counts: BTreeMap<String, u64>,
}
/// `GET /api/v1/imports/:id/items`の1件。取り込み対象の1ファイル分の結果。
///
/// - `source_root_name`・`relative_path`：どのフォルダのどのファイルか。絶対パスは返さない。
/// - `reason`：`skipped`・`failed`のときの理由（[`crate::error::ErrorCode`]の文字列）。
/// - `retryable`：同じファイルを再試行する意味があるか。
/// - `track_id`：取り込めた、または既存の曲と同じと判定した場合の曲ID。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportItem {
    pub id: Uuid,
    pub source_root_name: String,
    pub relative_path: String,
    pub status: ItemStatus,
    pub reason: Option<String>,
    pub retryable: bool,
    pub track_id: Option<Uuid>,
}
/// `POST /api/v1/imports`の応答（202）。始めたジョブのID。
/// すでに実行中のジョブがあれば、新しく始めずにそのジョブのIDを返す。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportAccepted {
    pub job_id: Uuid,
}
