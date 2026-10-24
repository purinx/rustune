//! 開発専用のUIギャラリー（Storybookの代わり）のエントリポイント。
//!
//! # 責務
//! - `/gallery`を開いたときは、ギャラリー（[`gallery::Gallery`]）を描画する。
//! - それ以外のURLでは、アプリ本体の画面（[`rustune_web::shell::AppShell`]）を描画する。
//!   ギャラリーの画面プレビューは、このURL（例：`/library`）をiframeで読み込む。
//!
//! # 責務外
//! 部品と画面の実装は`rustune-web`のlibraryにあり、ここでは使うだけ。
//! このcrateは開発専用で、アプリ本体（`rustune-web`）の成果物やserverの配信物には含まれない。
//!
//! # Rustの読みどころ
//! - `is_ok_and(...)`は、`Result`が`Ok`で、かつ中身が条件を満たすときだけ`true`を返す。
//!   `pathname()`はJS側の例外を`Err`で返すため、取得に失敗したときはアプリ本体を描画する。

mod gallery;

use leptos::{mount::mount_to_body, prelude::window};
use rustune_web::shell::AppShell;

fn main() {
    console_error_panic_hook::set_once();
    let is_gallery = window()
        .location()
        .pathname()
        .is_ok_and(|path| path.starts_with("/gallery"));
    if is_gallery {
        mount_to_body(gallery::Gallery);
    } else {
        mount_to_body(AppShell);
    }
}
