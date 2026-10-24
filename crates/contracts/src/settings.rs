//! 取り込み設定の契約：設定画面での取得と変更。
//!
//! # 処理フロー
//! 1. 設定画面が`GET /api/v1/settings`で[`Settings`]を取得し、値と`revision`を持つ。
//! 2. 保存するとき、変更した項目だけを入れた[`SettingsPatch`]を`PATCH /api/v1/settings`で送る。
//! 3. serverは`validate()`で範囲を確認し、続けて取り込み元フォルダの存在・権限・保存先との重なりを確認する。
//!    どれかが不正なら400（`invalid_input`か`invalid_source_root`）、revisionが違えば409。
//!    取り込みジョブの実行中は、取り込み元を変更できない（409、MODULE_DESIGN §4）。
//! 4. 保存できたら、新しい[`Settings`]を返す。
//!
use crate::{InvalidInput, Patch};
use serde::{Deserialize, Serialize};
/// `GET /api/v1/settings`の応答（`PATCH`の応答としても返す）。画面から変えられる取り込み設定。
/// 取り込み元の絶対パスは、この設定の境界でだけ公開する。
/// port・保存先・ログなど運用の設定はTOMLファイルが持ち、ここには含めない（MODULE_DESIGN §4）。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    /// 取り込み元フォルダの絶対パス。
    pub source_root: String,
    /// 自動取り込み（起動時と定期走査）を行うか。
    pub auto_import: bool,
    /// 定期走査の間隔（秒、30〜3600）。
    pub interval_seconds: u32,
    /// 1ファイルのサイズの上限（バイト、1 MiB〜2 GiB）。超えると`file_too_large`で対象外。
    pub max_file_bytes: u64,
    /// 保存先に必ず残す空き容量（バイト、1 GiB以上）。下回ると`storage_full`。
    pub reserve_free_bytes: u64,
    /// iTunes Search APIで使う国コード（英小文字2文字、例：`jp`）。
    pub storefront: String,
    /// 設定の更新番号。変更するときに`expected_revision`として送り返す。
    pub revision: u64,
}
/// `PATCH /api/v1/settings`のリクエスト。変更した項目だけを[`crate::Patch::Set`]で送る。
/// 設定には「取り込み値へ戻す」がないので、[`crate::Patch::Clear`]（null）は`validate()`で拒否する。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SettingsPatch {
    /// 変更前に読んだ[`Settings::revision`]。
    pub expected_revision: u64,
    /// 取り込み元フォルダの絶対パス。取り込みジョブの実行中は変更できない（409）。
    #[serde(default, skip_serializing_if = "Patch::is_keep")]
    pub source_root: Patch<String>,
    /// 自動取り込みの有無。
    #[serde(default, skip_serializing_if = "Patch::is_keep")]
    pub auto_import: Patch<bool>,
    /// 定期走査の間隔（秒、30〜3600）。
    #[serde(default, skip_serializing_if = "Patch::is_keep")]
    pub interval_seconds: Patch<u32>,
    /// 1ファイルの上限（バイト、1 MiB〜2 GiB）。
    #[serde(default, skip_serializing_if = "Patch::is_keep")]
    pub max_file_bytes: Patch<u64>,
    /// 保存先に残す空き容量（バイト、1 GiB以上）。
    #[serde(default, skip_serializing_if = "Patch::is_keep")]
    pub reserve_free_bytes: Patch<u64>,
    /// 国コード（英小文字2文字）。
    #[serde(default, skip_serializing_if = "Patch::is_keep")]
    pub storefront: Patch<String>,
}
impl SettingsPatch {
    /// 値の範囲だけを検証する：間隔30〜3600秒、上限1 MiB〜2 GiB、空き容量1 GiB以上、
    /// 国コードは英小文字2文字、パスは空でなくNUL文字を含まない。
    /// ファイルシステム上の包含関係と権限の検証は、serverのconfig・sourceが担う。
    /// （フォルダが存在するか、保存先と重なっていないかは、実際のファイルシステムを見ないとわからないため）
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
