//! ライブラリの表示と編集の契約：曲・アルバム・アーティスト・プレイリスト・お気に入り・除外。
//!
//! # 対応するAPI（`/api/v1`）
//! | 操作 | API | リクエスト | レスポンス |
//! | --- | --- | --- | --- |
//! | 曲の検索・一覧 | `GET /tracks` | 検索語・[`crate::Pagination`] | [`crate::Page`]<[`TrackSummary`]> |
//! | 曲の詳細 | `GET /tracks/:id` | なし | [`TrackDetail`] |
//! | 曲の編集 | `PATCH /tracks/:id` | [`MetadataPatch`] | [`TrackDetail`] |
//! | 曲の削除 | `DELETE /tracks/:id` | `expected_revision` | 202と削除中の状態 |
//! | お気に入り | `PUT`・`DELETE /tracks/:id/favorite` | なし | 更新後の状態 |
//! | アルバム・アーティスト | `GET /albums`・`/artists`、`/:key` | [`crate::Pagination`] | [`AlbumSummary`]・[`ArtistSummary`]と曲の一覧 |
//! | プレイリスト | `POST /playlists`、`PATCH /playlists/:id`、`PUT /playlists/:id/items` | [`CreatePlaylist`]・[`RenamePlaylist`]・[`ReplacePlaylistItems`] | [`Playlist`] |
//! | 除外 | `GET /exclusions`、`DELETE /exclusions/:id` | なし | [`Exclusion`] |
//!
//! 曲の削除の状態、`GET /session`・`GET /state`（同期用）など、設計にあってまだここにない型は、
//! HTTPを実装するタスク11で追加する。
//!
//! # 処理フロー（曲を編集する例）
//! ```mermaid
//! sequenceDiagram
//!     participant ui as UI（web）
//!     participant api as server
//!     participant db as DB
//!     ui->>api: GET /tracks/:id
//!     api->>db: 読み取り
//!     api-->>ui: TrackDetail（revision=5）
//!     ui->>api: PATCH /tracks/:id MetadataPatch（expected_revision=5、変更した項目だけ）
//!     api->>api: validate()（不正なら400 invalid_input）
//!     api->>db: revisionが5なら更新してrevision=6
//!     alt revisionが一致
//!         api-->>ui: 新しいTrackDetail（revision=6）
//!     else 他で先に更新されていた
//!         api-->>ui: 409 revision_conflict → UIは読み直して確認を求める
//!     end
//! ```
//!
use crate::{InvalidInput, Patch, validate_number, validate_text};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 一覧に出す1曲分の情報。`GET /api/v1/tracks`（検索・一覧）、アルバム・アーティスト・お気に入りの各一覧で、
/// [`crate::Page`]の`items`として返す。取り込みが完了した（ready状態の）曲だけが含まれる。
///
/// - `title`・`artist`・`album`：手動編集・補完・取り込み値のうち、有効な値（どれを使ったかは[`TrackDetail`]で返す）。
/// - `artwork_id`：`GET /api/v1/artwork/:artwork_id`で画像を取るためのID。`None`なら既定画像を表示する。
/// - `revision`：この曲の更新番号。編集（[`MetadataPatch`]）のときに`expected_revision`として送り返す。
///
/// 再生は`GET /api/v1/media/:id`（`id`を使う）。保存先のパスは含めない。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrackSummary {
    pub id: Uuid,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration_ms: u64,
    pub artwork_id: Option<Uuid>,
    pub favorite: bool,
    pub revision: u64,
}
/// 曲の取り込み元。取り込み元フォルダの名前と、その中の相対パスだけを返す（絶対パスは返さない）。
/// 同じ音源が別のパスにも見つかった場合は、[`TrackDetail::sources`]に複数並ぶ。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Source {
    pub root_name: String,
    pub relative_path: String,
    pub imported_at: String,
}
/// 表示している値がどこから来たか。編集画面で「手動で変更済み」などを示し、取り込み値へ戻す操作に使う。
/// - `manual`：利用者が編集した値
/// - `enriched`：iTunes Search APIの補完で反映した値
/// - `imported`：音源ファイルのタグから読んだ値
/// - `fallback`：タグがなく、ファイル名などから補った値
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MetadataOrigin {
    Manual,
    Enriched,
    Imported,
    Fallback,
}
/// 項目ごとの[`MetadataOrigin`]。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MetadataOrigins {
    pub title: MetadataOrigin,
    pub artist: MetadataOrigin,
    pub album: MetadataOrigin,
    pub album_artist: MetadataOrigin,
    pub disc_number: MetadataOrigin,
    pub track_number: MetadataOrigin,
}
/// `GET /api/v1/tracks/:id`の応答。曲の詳細・編集画面で使う。
/// 一覧用の[`TrackSummary`]に、アルバムアーティスト・番号・取り込み元・各値の由来を加えたもの。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrackDetail {
    pub summary: TrackSummary,
    pub album_artist: Option<String>,
    pub disc_number: Option<u16>,
    pub track_number: Option<u16>,
    pub sources: Vec<Source>,
    pub metadata_origins: MetadataOrigins,
}

/// `PATCH /api/v1/tracks/:id`のリクエスト。曲のメタデータを部分的に編集する。
///
/// 処理フロー：UIが[`TrackDetail`]を読む → 変更した項目だけを[`crate::Patch::Set`]にし、
/// 取り込み値へ戻す項目を[`crate::Patch::Clear`]にして、読んだときの`revision`を`expected_revision`に入れて送る →
/// serverは`validate()`（文字列は1〜512文字、番号は1〜999）の後、revisionが一致すれば保存して新しい値を返す。
/// 一致しなければ`revision_conflict`（409）。`deny_unknown_fields`で、編集できない項目を送るとエラーになる。
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MetadataPatch {
    pub expected_revision: u64,
    #[serde(default, skip_serializing_if = "Patch::is_keep")]
    pub title: Patch<String>,
    #[serde(default, skip_serializing_if = "Patch::is_keep")]
    pub artist: Patch<String>,
    #[serde(default, skip_serializing_if = "Patch::is_keep")]
    pub album: Patch<String>,
    #[serde(default, skip_serializing_if = "Patch::is_keep")]
    pub album_artist: Patch<String>,
    #[serde(default, skip_serializing_if = "Patch::is_keep")]
    pub disc_number: Patch<u16>,
    #[serde(default, skip_serializing_if = "Patch::is_keep")]
    pub track_number: Patch<u16>,
}
impl MetadataPatch {
    pub fn validate(mut self) -> Result<Self, InvalidInput> {
        for patch in [
            &mut self.title,
            &mut self.artist,
            &mut self.album,
            &mut self.album_artist,
        ] {
            if let Patch::Set(text) = patch {
                *text = validate_text(text, 512)?;
            }
        }
        for patch in [&self.disc_number, &self.track_number] {
            if let Patch::Set(n) = patch {
                validate_number(*n)?;
            }
        }
        Ok(self)
    }
}
/// プレイリストの1項目。`id`は項目ごとの独自UUIDで、同じ曲（`track_id`）を複数回入れても区別できる。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlaylistItem {
    pub id: Uuid,
    pub track_id: Uuid,
}
/// プレイリスト。`GET /api/v1/playlists/:id`の応答で、作成・改名・並べ替えの応答としても返す。
/// `items`の順番が再生順。`revision`は改名・並べ替えのときに`expected_revision`として送り返す。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Playlist {
    pub id: Uuid,
    pub name: String,
    pub revision: u64,
    pub items: Vec<PlaylistItem>,
}
/// `POST /api/v1/playlists`のリクエスト。名前はtrim後1〜120文字。成功すると201と[`Playlist`]を返す。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreatePlaylist {
    pub name: String,
}
impl CreatePlaylist {
    pub fn validate(mut self) -> Result<Self, InvalidInput> {
        self.name = validate_text(&self.name, 120)?;
        Ok(self)
    }
}
/// `PATCH /api/v1/playlists/:id`のリクエスト。名前の検証は[`CreatePlaylist`]と同じ。
/// `expected_revision`が現在のrevisionと違えば409。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RenamePlaylist {
    pub expected_revision: u64,
    pub name: String,
}
impl RenamePlaylist {
    pub fn validate(mut self) -> Result<Self, InvalidInput> {
        self.name = validate_text(&self.name, 120)?;
        Ok(self)
    }
}
/// `PUT /api/v1/playlists/:id/items`のリクエスト。追加・削除・並べ替えのすべてを、
/// 「変更後の全項目を順番どおりに送る」ことで表す（差分ではなく全体を置き換える）。
///
/// - 新しく追加する項目には、UIが新しい`id`を振る。既存の項目は元の`id`のまま送る。
/// - `validate()`は、nilのUUIDと、同じ`id`の重複を拒否する。
/// - 知らない項目のidや、ready状態でない曲を含んでいないかは、DBを見る必要があるのでserverが確認する。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplacePlaylistItems {
    pub expected_revision: u64,
    pub items: Vec<PlaylistItem>,
}
impl ReplacePlaylistItems {
    pub fn validate(&self) -> Result<(), InvalidInput> {
        let mut ids = std::collections::HashSet::new();
        if self
            .items
            .iter()
            .any(|item| item.id.is_nil() || item.track_id.is_nil() || !ids.insert(item.id))
        {
            return Err(InvalidInput);
        }
        Ok(())
    }
}
/// `GET /api/v1/albums`の1件。`id`はアルバムをまとめるためのキー（ストアのIDか、表示名から作った値）で、
/// 詳細の`GET /api/v1/albums/:key`に使う。メタデータの編集でアルバムが変わると、キーも変わることがある。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AlbumSummary {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub artwork_id: Option<Uuid>,
    pub track_count: u64,
}
/// `GET /api/v1/artists`の1件。`id`は詳細の`GET /api/v1/artists/:key`に使うキー。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtistSummary {
    pub id: String,
    pub name: String,
    pub track_count: u64,
}
/// `GET /api/v1/exclusions`の1件。ライブラリから削除し、再び取り込まないようにした曲。
/// `DELETE /api/v1/exclusions/:id`で除外を解除すると、次の走査で再び取り込まれる。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Exclusion {
    pub track_id: Uuid,
    pub title: String,
    pub excluded_at: String,
}
