//! 公開する通信用の表現。DBの行や音源の生タグをそのままserializeしない。

use serde::{Deserialize, Serialize};

pub mod enrichment;
pub mod error;
pub mod imports;
pub mod library;
pub mod settings;

/// フィールド省略は手動値を維持、nullは消去、値は設定を表す。
/// この型を使うフィールドには必ずserde(default, skip_serializing_if)を付ける。
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
/// trim後のUnicode scalar数。UTF-8のバイト数や書記素数ではない。
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pagination {
    pub offset: u64,
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
    pub fn validate(self) -> Result<Self, InvalidInput> {
        if (1..=100).contains(&self.limit)
            && self.offset.checked_add(u64::from(self.limit)).is_some()
        {
            Ok(self)
        } else {
            Err(InvalidInput)
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub total: u64,
    pub offset: u64,
    pub limit: u16,
    pub library_revision: u64,
}

/// 開発基盤用の接続確認情報。ライブラリが利用可能かどうかは表さない。
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct HealthResponse {
    pub service: String,
    pub version: String,
    pub stage: String,
}
