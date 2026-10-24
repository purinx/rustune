//! Rustune Web UI（WASM）のエントリポイント。
//!
//! # 責務
//! - ブラウザがWASMを読み込んだときに最初に呼ばれる`main`を提供し、画面全体を`<body>`へ描画（mount）する。
//! - 開発ビルドで`/gallery`を開いたときだけ、アプリ本体の代わりにUIギャラリー（`gallery`モジュール）を描画する。
//! - panic（Rustの回復不能なエラー）の内容をブラウザのコンソールに出すフックを、最初に設定する。
//!
//! # 責務外
//! 画面遷移と各画面の表示は[`shell`]、共通部品は[`components`]が担う。ここには画面ロジックを書かない。
//!
//! # 処理の流れ
//! 1. Trunkがビルドした`index.html`が、wasm-bindgenの生成したJSを通じてWASMを読み込み、`main`を呼ぶ。
//! 2. `console_error_panic_hook::set_once()`で、panic時のメッセージを`console.error`へ出す。
//!    これがないと、WASMのpanicはコンソールに「unreachable」としか表示されない。
//! 3. `mount_to_body`にルートのコンポーネントを渡すと、leptosがDOMを生成する。以後はsignal（`components`モジュールの説明を参照）の
//!    変更に応じて、必要な部分だけを更新する。`main`自体はすぐに終わるが、描画したUIとイベント処理は残り続ける。
//!
//! # Rustの読みどころ
//! - `mod components;`は、同じcrateのモジュール（`src/components/mod.rs`）を読み込む宣言。
//! - `#[cfg(debug_assertions)]`は「debugビルドのときだけこの項目をコンパイルする」条件付きコンパイル。
//!   `trunk serve`はdebug、`trunk build --release`はreleaseなので、ギャラリーは本番の成果物に入らない。
//!   `mod gallery;`と`if`文の両方に付けているのは、片方だけだとreleaseでコンパイルエラー（存在しない`gallery`の参照）か
//!   未使用コードの警告（CIでは`-D warnings`でエラー）になるため。
//! - `is_ok_and(...)`は、`Result`が`Ok`で、かつ中身が条件を満たすときだけ`true`を返す。
//!   `pathname()`はJS側の例外を`Err`で返すため、取得に失敗したときは通常のアプリを描画する。

mod components;
#[cfg(debug_assertions)]
mod gallery;
mod shell;

use leptos::mount::mount_to_body;

fn main() {
    console_error_panic_hook::set_once();
    #[cfg(debug_assertions)]
    if leptos::prelude::window()
        .location()
        .pathname()
        .is_ok_and(|path| path.starts_with("/gallery"))
    {
        mount_to_body(gallery::Gallery);
        return;
    }
    mount_to_body(shell::AppShell);
}
