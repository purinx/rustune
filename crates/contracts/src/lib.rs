//! ブラウザのUI（`rustune-web`）とサーバー（`rustune-server`）の間の、HTTP API（`/api/v1`）の契約。
//!
//! # 何の契約か
//! UIとサーバーは別プロセスで、HTTPのJSONだけでやり取りする。このcrateには、そのJSONの形
//! （リクエスト・レスポンス・エラーの型）と、受け取った値が正しいかの検証（`validate`）だけを置く。
//! 両方のcrateが同じ型を使ってserialize・deserializeするので、片方だけ形を変えるとコンパイルかテストで気づける。
//! 型を変えることはAPIを変えることなので、承認済みの設計（MODULE_DESIGN §2・§12）と合わせて更新する。
//!
//! ```mermaid
//! flowchart LR
//!     web["rustune-web（WASM）"] -- "JSON：contractsの型" --> server["rustune-server（Axum）"]
//!     server -- "JSON：contractsの型" --> web
//!     web -. "コンパイル時に依存" .-> contracts["rustune-contracts"]
//!     server -. "コンパイル時に依存" .-> contracts
//!     server --> domain["rustune-domain（業務の型）"]
//!     server --> db[("SQLite")]
//! ```
//!
//! # 置くもの・置かないもの
//! - 置く：通信用の型（DTO）、文字数や範囲などの入力制約、エラーcode。依存はserdeとuuidだけ。
//! - 置かない：業務ルールや状態遷移（`rustune-domain`）、DBの行の型（`server::db`）、HTTPやファイルの処理。
//! - 外へ出さない情報：購入者タグ、保存先のパス、OSやパーサーの生のエラー文字列。これらの型はここに定義しない。
//!   取り込み元の絶対パスだけは、設定画面のために[`settings::Settings`]で返す。
//!
//! serverは、DBの行やdomainの型をそのまま返さず、必ずこのcrateの型へ変換してから返す。
//! こうすることで、内部の型を変えてもAPIの形は変わらず、公開してはいけない項目が誤って出ることもない。
//!
//! # JSONでの表し方
//! IDはUUIDの文字列、時刻はUTCのRFC 3339文字列、時間はミリ秒、サイズはバイト、revisionは0以上の整数。
//!
//! # 共通の処理フロー
//! 1. **読み取り**：UIがGETする → serverがDB等から読み、このcrateの型に変換して返す →
//!    UIはその型にdeserializeして表示する。一覧は[`Pagination`]で範囲を指定し、[`Page`]で受け取る。
//! 2. **変更**：UIが`expected_revision`（読み取ったときのrevision）を付けて送る →
//!    serverがdeserializeし、`validate()`で入力を検証する（不正なら400）→
//!    DBの現在のrevisionと比べ、違えば`revision_conflict`（409）を返す（楽観的ロック）→
//!    一致すれば更新し、revisionを増やして新しい値を返す。409を受けたUIは読み直して、利用者に確認を求める。
//! 3. **同期**：[`Page`]などが返す`library_revision`は、ライブラリ全体の更新番号。
//!    UIは定期的に状態を取得し、この値が変わっていたら表示中の一覧を読み直す（MODULE_DESIGN §14）。
//! 4. **失敗**：serverは[`error::ApiError`]を返す。UIは`code`で表示を決め、知らないcodeなら`message`をそのまま出す。
//!
//! # モジュール
//! - [`library`]：曲・アルバム・アーティスト・プレイリスト・除外の表示と編集
//! - [`imports`]：取り込みジョブの開始と進み具合
//! - [`settings`]：取り込み設定の取得と変更
//! - [`enrichment`]：iTunes Search APIでのメタデータ補完
//! - [`error`]：失敗時の応答とエラーcode
//! - このファイル：複数のモジュールで使う部品（[`Patch`]、[`Pagination`]、[`Page`]、文字列・数値の検証）

use serde::{Deserialize, Serialize};

pub mod enrichment;
pub mod error;
pub mod imports;
pub mod library;
pub mod settings;

/// 部分更新（PATCH）の各フィールドを「維持・設定・消去」の3状態で表す。
/// フィールド省略は手動値を維持、nullは消去、値は設定を表す。
/// この型を使うフィールドには必ずserde(default, skip_serializing_if)を付ける。
///
/// 「消去」は空文字にすることではなく、手動で編集した値を捨てて取り込み時の値へ戻すことを意味する。
/// JSONの`null`と「キーがない」を区別する必要があるため、`Option<T>`ではなくこの型を使う。
///
/// | JSON | 値 | serverの処理 |
/// | --- | --- | --- |
/// | キーなし | `Keep` | 変更しない |
/// | `"title": "曲名"` | `Set("曲名")` | 手動値として保存する |
/// | `"title": null` | `Clear` | 手動値を消し、取り込み値へ戻す |
///
/// `Keep`はキーごと省略して表すので、serializeしようとするとエラーにする（`skip_serializing_if`で省略される前提）。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Patch<T> {
    /// 変更しない（JSONではキーを省略）。
    #[default]
    Keep,
    /// この値に設定する（JSONでは値）。
    Set(T),
    /// 手動の値を消して取り込み値へ戻す（JSONでは`null`）。
    Clear,
}
impl<T> Patch<T> {
    pub fn is_keep(&self) -> bool {
        matches!(self, Self::Keep)
    }
}
impl<T: Serialize> Serialize for Patch<T> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Set(value) => value.serialize(serializer),
            Self::Clear => serializer.serialize_none(),
            Self::Keep => Err(serde::ser::Error::custom("Keep must be omitted")),
        }
    }
}
impl<'de, T: Deserialize<'de>> Deserialize<'de> for Patch<T> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(Option::<T>::deserialize(deserializer)?.map_or(Self::Clear, Self::Set))
    }
}

/// 各`validate`が返す入力不正。serverはこれを`ErrorCode::InvalidInput`（HTTP 400）の応答に変換する。
/// どのフィールドが不正かは持たない。UIは送信前に同じ`validate`を呼んで、入力欄で先に知らせられる。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InvalidInput;
/// 文字列の共通検証：前後の空白を除き、1〜`max`文字なら除去後の文字列を返す。
/// 文字数はtrim後のUnicode scalar数。UTF-8のバイト数や書記素数ではない。
/// （例：「が」は1文字、絵文字の合成列は複数文字として数える）
pub fn validate_text(value: &str, max: usize) -> Result<String, InvalidInput> {
    let value = value.trim();
    if (1..=max).contains(&value.chars().count()) {
        Ok(value.to_owned())
    } else {
        Err(InvalidInput)
    }
}
/// ディスク番号・曲順の共通検証：1〜999を受け付ける。
pub fn validate_number(value: u16) -> Result<(), InvalidInput> {
    if (1..=999).contains(&value) {
        Ok(())
    } else {
        Err(InvalidInput)
    }
}

/// 一覧取得のページ指定。曲・アルバム・取り込み結果などの一覧APIで、どの範囲を返すかをUIが指定する。
/// 応答は[`Page`]で返る。省略したフィールドは`Default`（offset 0・limit 50）で補い、
/// 明示した値は`validate`で検証する。
///
/// - `#[serde(default)]`：JSONで省略されたフィールドを、下の`Default`実装の値で埋める。
///   `{}`ならoffset 0・limit 50、`{"offset":100}`ならlimitだけ50になる。
/// - `#[serde(deny_unknown_fields)]`：定義にないフィールド（`{"page":2}`など）を含むJSONはエラーにする。
///   名前を打ち間違えたときに、黙って既定値で処理されるのを防ぐ。
/// - deserializeの時点では範囲を検査しない。受け取った側が`validate()`を呼び、
///   limitが1〜100、offsetが0〜1,000,000であることを確認する。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Pagination {
    /// 先頭から何件を飛ばすか（0〜1,000,000、省略時は0）。
    pub offset: u64,
    /// 返す最大件数（1〜100、省略時は50）。
    pub limit: u16,
}
impl Default for Pagination {
    fn default() -> Self {
        Self {
            offset: 0,
            limit: 50,
        }
    }
}
impl Pagination {
    /// 受け付けるoffsetの上限（MODULE_DESIGN.md §11）。
    pub const MAX_OFFSET: u64 = 1_000_000;
    /// limitの上限（MODULE_DESIGN.md §11）。
    pub const MAX_LIMIT: u16 = 100;

    pub fn validate(self) -> Result<Self, InvalidInput> {
        if (1..=Self::MAX_LIMIT).contains(&self.limit) && self.offset <= Self::MAX_OFFSET {
            Ok(self)
        } else {
            Err(InvalidInput)
        }
    }
}
/// 一覧APIの応答。`items`に`offset`から最大`limit`件を入れ、全体の件数と更新番号を添える。
///
/// serverは、件数・一覧・`library_revision`を同じ読み取りトランザクションから返す。
/// そのため、途中で取り込みや編集があっても、3つの値が食い違わない（MODULE_DESIGN §11）。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Page<T> {
    /// このページの要素。
    pub items: Vec<T>,
    /// 条件に一致する全体の件数。UIはページ送りの表示に使う。
    pub total: u64,
    /// このページの先頭位置（リクエストの値）。
    pub offset: u64,
    /// このページの最大件数（リクエストの値）。
    pub limit: u16,
    /// ライブラリ全体の更新番号。UIは、同期で取得した値と比べて変わっていたら一覧を読み直す。
    pub library_revision: u64,
}

/// `GET /api/v1/health`の応答。開発基盤用の接続確認情報。ライブラリが利用可能かどうかは表さない。
/// （DBと起動時復旧の完了は、設計上`GET /api/v1/health/ready`で別に返す）
/// 設定画面の「接続を確認」が、`service == "rustune"`で接続先がRustuneのサーバーかを判定する。
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct HealthResponse {
    /// サービス名。Rustuneのサーバーなら`"rustune"`。
    pub service: String,
    /// サーバーのバージョン。
    pub version: String,
    /// 実装の段階。現在の開発基盤では`"foundation"`。
    pub stage: String,
}
