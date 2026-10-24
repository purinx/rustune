//! 各routeに表示する画面（今はすべて準備中の表示）。1画面1ファイル。
//!
//! 実際のデータを表示する画面は、タスク13が`web::pages`（`crates/web/src/pages/`）として作り、ここを置き換える。
//! 設定画面（[`Settings`]）だけは、サーバーの接続確認で実際にAPIを呼ぶ。

mod collection;
mod exclusions;
mod imports;
mod library;
mod not_found;
mod settings;

pub(crate) use collection::Collection;
pub(crate) use exclusions::Exclusions;
pub(crate) use imports::Imports;
pub(crate) use library::Library;
pub(crate) use not_found::NotFound;
pub(crate) use settings::Settings;
