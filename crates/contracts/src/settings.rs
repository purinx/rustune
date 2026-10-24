use crate::{InvalidInput, Patch};
use serde::{Deserialize, Serialize};
/// 取り込み元の絶対パスは、この設定の境界でだけ公開する。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    pub source_root: String,
    pub auto_import: bool,
    pub interval_seconds: u32,
    pub max_file_bytes: u64,
    pub reserve_free_bytes: u64,
    pub storefront: String,
    pub revision: u64,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SettingsPatch {
    pub expected_revision: u64,
    #[serde(default, skip_serializing_if = "Patch::is_keep")]
    pub source_root: Patch<String>,
    #[serde(default, skip_serializing_if = "Patch::is_keep")]
    pub auto_import: Patch<bool>,
    #[serde(default, skip_serializing_if = "Patch::is_keep")]
    pub interval_seconds: Patch<u32>,
    #[serde(default, skip_serializing_if = "Patch::is_keep")]
    pub max_file_bytes: Patch<u64>,
    #[serde(default, skip_serializing_if = "Patch::is_keep")]
    pub reserve_free_bytes: Patch<u64>,
    #[serde(default, skip_serializing_if = "Patch::is_keep")]
    pub storefront: Patch<String>,
}
impl SettingsPatch {
    /// ファイルシステム上の包含関係と権限の検証は、serverのconfig・sourceが担う。
    pub fn validate(&self) -> Result<(), InvalidInput> {
        if matches!(&self.source_root, Patch::Clear)
            || matches!(self.auto_import, Patch::Clear)
            || matches!(self.interval_seconds, Patch::Clear)
            || matches!(self.max_file_bytes, Patch::Clear)
            || matches!(self.reserve_free_bytes, Patch::Clear)
            || matches!(&self.storefront, Patch::Clear)
            || matches!(&self.source_root, Patch::Set(p) if p.trim().is_empty() || p.contains('\0'))
            || matches!(self.interval_seconds, Patch::Set(n) if !(30..=3600).contains(&n))
            || matches!(self.max_file_bytes, Patch::Set(n) if !(1_048_576..=2_147_483_648).contains(&n))
            || matches!(self.reserve_free_bytes, Patch::Set(n) if n < 1_073_741_824)
            || matches!(&self.storefront, Patch::Set(s) if s.len() != 2 || !s.bytes().all(|c| c.is_ascii_lowercase()))
        {
            return Err(InvalidInput);
        }
        Ok(())
    }
}
