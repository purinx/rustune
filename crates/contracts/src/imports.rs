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
    /// 取り込み画面のボタンで始めた。
    Manual,
    /// serverの起動時に自動で始めた。
    Startup,
    /// 設定の間隔ごとの定期走査で始めた。
    Scheduled,
}
/// ジョブ全体の状態。`queued`→`running`→`completed`か`failed`。
/// 個々のファイルが失敗してもジョブは`completed`になり、`failed`の件数で表す。
/// ジョブが`failed`になるのは、取り込み元フォルダ全体を読めない、空き容量が尽きた、など続けられない場合。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    /// 予約済みで、まだ始まっていない。
    Queued,
    /// 実行中。
    Running,
    /// 最後まで処理した。一部のファイルが失敗していても`completed`になる。
    Completed,
    /// 続けられずに止まった（取り込み元全体を読めない、空き容量が尽きた、など）。
    Failed,
}
/// 実行中の段階。`enumerating`は対象ファイルを数えている途中（`total`はまだ`None`）、
/// `processing`は件数が確定して1件ずつ処理している段階。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportPhase {
    /// 取り込み元の対象ファイルを数えている途中。[`ImportJob::total`]は`None`。
    Enumerating,
    /// 件数が確定し、1件ずつ処理している。
    Processing,
}
/// ファイル1件の状態。`pending`→`checking`（解析中）→`copying`（保存先へコピー中）→`imported`。
/// 途中で対象外とわかれば`skipped`、処理に失敗すれば`failed`になり、理由は[`ImportItem::reason`]に入る。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemStatus {
    /// まだ処理していない。
    Pending,
    /// 音源を解析している（形式・購入曲かの確認）。
    Checking,
    /// 保存先へコピーしている。
    Copying,
    /// 取り込めた。既存の曲と同じ音源だった場合も含む。
    Imported,
    /// 対象外だった（[`ImportItem::reason`]に理由）。
    Skipped,
    /// 処理に失敗した（[`ImportItem::reason`]に理由）。
    Failed,
}
/// `GET /api/v1/imports`・`/imports/:id`の応答。取り込みジョブ1回分の進み具合と集計。
/// 取り込み画面は、取り込み中これを2秒ごとに取得して進み具合を表示する。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportJob {
    /// ジョブのID。
    pub id: Uuid,
    /// 取り込みを始めたきっかけ。
    pub trigger: ImportTrigger,
    /// ジョブ全体の状態。
    pub status: JobStatus,
    /// 実行中の段階。
    pub phase: ImportPhase,
    /// 対象ファイルの総数。数えている途中（`enumerating`）は`None`。
    pub total: Option<u64>,
    /// 処理が終わった件数。常に`added + skipped + failed`と一致する。
    pub processed: u64,
    /// 新しく取り込んだ件数。
    pub added: u64,
    /// 対象外だった件数。
    pub skipped: u64,
    /// 失敗した件数。
    pub failed: u64,
    /// 理由ごとの件数。キーは[`crate::error::ErrorCode`]の文字列（例：`purchase_unverified`）。
    /// `BTreeMap`なので、JSONでのキーの順番が毎回同じになる。
    pub reason_counts: BTreeMap<String, u64>,
}
/// `GET /api/v1/imports/:id/items`の1件。取り込み対象の1ファイル分の結果。
/// [`crate::Page`]で、ジョブごとにページングして返す。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportItem {
    /// この結果のID。
    pub id: Uuid,
    /// 取り込み元フォルダの表示名。絶対パスではない。
    pub source_root_name: String,
    /// 取り込み元フォルダの中での相対パス。
    pub relative_path: String,
    /// このファイルの状態。
    pub status: ItemStatus,
    /// `skipped`・`failed`のときの理由（[`crate::error::ErrorCode`]の文字列）。それ以外は`None`。
    pub reason: Option<String>,
    /// 同じファイルを再試行する意味があるか。
    pub retryable: bool,
    /// 取り込めた、または既存の曲と同じと判定した場合の曲ID。
    pub track_id: Option<Uuid>,
}
/// `POST /api/v1/imports`の応答（202）。始めたジョブのID。
/// すでに実行中のジョブがあれば、新しく始めずにそのジョブのIDを返す。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportAccepted {
    /// 始めたジョブ（すでに実行中ならそのジョブ）のID。`GET /imports/:job_id`で進み具合を取得する。
    pub job_id: Uuid,
}
