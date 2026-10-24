//! Public wire representations. Never serialize database rows or raw audio tags.

use serde::{Deserialize, Serialize};

pub mod enrichment;
pub mod error;
pub mod imports;
pub mod library;
pub mod settings;

/// Missing field keeps a manual override; null clears it; a value sets it.
/// Every field using this type must include serde(default, skip_serializing_if).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Patch<T> {
    #[default]
    Keep,
    Set(T),
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InvalidInput;
/// Trimmed Unicode scalar count, not UTF-8 bytes or grapheme count.
pub fn validate_text(value: &str, max: usize) -> Result<String, InvalidInput> {
    let value = value.trim();
    if (1..=max).contains(&value.chars().count()) {
        Ok(value.to_owned())
    } else {
        Err(InvalidInput)
    }
}
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

/// Connectivity information for the development foundation, not library readiness.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct HealthResponse {
    pub service: String,
    pub version: String,
    pub stage: String,
}
