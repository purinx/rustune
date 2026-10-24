//! Rustune Web UI（WASM）のエントリポイント。
//!
//! # 責務
//! - ブラウザがWASMを読み込んだときに最初に呼ばれる`main`を提供し、画面全体を`<body>`へ描画（mount）する。
//! - panic（Rustの回復不能なエラー）の内容をブラウザのコンソールに出すフックを、最初に設定する。
//!
//! # 責務外
//! 画面遷移と各画面の表示は[`rustune_web::shell`]、共通部品は[`rustune_web::components`]が担う
//! （同じpackageのlibrary、`src/lib.rs`）。ここには画面ロジックを書かない。
//! UIギャラリーは別crateの`rustune-stories`（`crates/stories`）にある。
//!
//! # 処理の流れ
//! 1. Trunkがビルドした`index.html`が、wasm-bindgenの生成したJSを通じてWASMを読み込み、`main`を呼ぶ。
//! 2. `console_error_panic_hook::set_once()`で、panic時のメッセージを`console.error`へ出す。
//!    これがないと、WASMのpanicはコンソールに「unreachable」としか表示されない。
//! 3. `mount_to_body`にルートのコンポーネントを渡すと、leptosがDOMを生成する。以後はsignal（`components`モジュールの説明を参照）の
//!    変更に応じて、必要な部分だけを更新する。`main`自体はすぐに終わるが、描画したUIとイベント処理は残り続ける。
//!
//! # Rustの読みどころ
//! - `rustune_web::shell::AppShell`は、同じpackageのlibrary（crate名`rustune_web`）の項目を参照するパス。
//!   package名の`-`は、Rustのコード上では`_`になる。

use leptos::mount::mount_to_body;
use rustune_web::shell::AppShell;

fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(AppShell);
}
