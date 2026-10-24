//! メタデータ補完の契約：iTunes Search APIで候補を探し、選んだ候補を曲へ反映する。
//!
//! # 処理フロー
//! ```mermaid
//! sequenceDiagram
//!     participant ui as UI（編集画面）
//!     participant api as server
//!     participant itunes as iTunes Search API
//!     ui->>api: POST /enrichment/search SearchRequest
//!     api->>itunes: 検索（間隔と同時数を制限）
//!     itunes-->>api: 結果
//!     api->>api: 許可した項目だけ取り出し、候補をメモリに10分保存
//!     api-->>ui: Candidateの一覧（idで候補を指す）
//!     ui->>api: POST /enrichment/apply ApplyRequest（candidate_id・項目・revision）
//!     api->>api: 保存した候補から反映（画像はserverが取得・検証）
//!     api-->>ui: 更新後の曲（409ならrevision競合）
//! ```
//! ブラウザは外部サービスと直接通信しない。適用のときも、URLや外部の生のJSONは受け取らず、候補のIDだけを受け取る。
//! こうすることで、ブラウザから任意のURLをserverに取りに行かせることができない。
//!
use crate::{InvalidInput, validate_text};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
/// `POST /api/v1/enrichment/search`のリクエスト。検索語はtrim後1〜512文字。
/// serverはこの検索語でiTunes Search APIを呼ぶ。ファイルのパスや購入者の情報は外部へ送らない。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SearchRequest {
    /// 検索語。trim後1〜512文字。
    pub query: String,
}
impl SearchRequest {
    pub fn validate(mut self) -> Result<Self, InvalidInput> {
        self.query = validate_text(&self.query, 512)?;
        Ok(self)
    }
}
/// 検索結果の候補1件。`POST /api/v1/enrichment/search`の応答として複数返す。
/// ブラウザからプレビューURLや任意の画像URLは受け付けない。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Candidate {
    /// serverのメモリに保存した候補を指すID。適用のときはこのIDだけを送る。候補は10分で期限切れになる。
    pub id: Uuid,
    /// 候補の曲名。
    pub title: String,
    /// 候補のアーティスト名。
    pub artist: String,
    /// 候補のアルバム名。
    pub album: String,
    /// iTunes Storeのページ。出典として画面に表示する。
    pub store_url: String,
    /// 候補に画像があるか。画像のURL自体はブラウザへ渡さない。
    pub artwork_available: bool,
}
/// 候補から反映する項目。`artwork`はジャケット画像。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnrichmentField {
    /// 曲名。
    Title,
    /// アーティスト名。
    Artist,
    /// アルバム名。
    Album,
    /// アルバムアーティスト。
    AlbumArtist,
    /// ディスク番号。
    DiscNumber,
    /// 曲順。
    TrackNumber,
    /// ジャケット画像。serverが候補の画像を取得・検証して保存する。
    Artwork,
}
/// `POST /api/v1/enrichment/apply`のリクエスト。選んだ候補の項目を曲へ反映する。
/// serverはメモリに保存した候補から値を取り出す。ブラウザが送るのは候補のIDと項目名だけ。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApplyRequest {
    /// 反映先の曲のID。
    pub track_id: Uuid,
    /// 選んだ候補の[`Candidate::id`]。期限切れなら再検索を案内する。
    pub candidate_id: Uuid,
    /// 反映前に読んだ曲の`revision`。違えば409。
    pub expected_revision: u64,
    /// 候補から反映する項目。
    pub fields: Vec<EnrichmentField>,
    /// 反映の前に手動編集を消す項目。手動で編集した項目は、ここで明示しない限り上書きしない。
    pub clear_manual_fields: Vec<EnrichmentField>,
}
