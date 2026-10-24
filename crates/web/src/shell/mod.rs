//! アプリ全体の骨格（AppShell）と、各画面のroute定義。
//!
//! # 責務
//! - URLと画面の対応（routing）を定義する。URLの管理とブラウザの戻る・進むは`leptos_router`が行う。
//! - 画面を移っても消えない部分を保持する：サイドバー（狭い幅では下部ナビゲーション）、固定再生領域、
//!   「Rustuneについて」ダイアログ。
//! - 画面が変わるたびにウィンドウタイトルとh1を更新し、h1へフォーカスを移す。
//!   スクリーンリーダーの利用者に、画面が変わったことを伝えるため。
//! - 今の各画面は「準備中」の表示だけ。実際にAPIを呼ぶのは、設定画面のサーバー接続確認だけ。
//!
//! # 責務外
//! 楽曲データの取得と再生（タスク12・13）は持たない。見た目の値は`style.css`のトークンが決める。
//!
//! # 構成
//! ```text
//! AppShell ── Router（URLを監視）
//!   └ Shell
//!       ├ Sidebar       ：ブランド、NAVのリンク、「Rustuneについて」ボタン
//!       ├ main          ：見出し（h1）＋ AppRoutes（URLに一致する画面を1つ描画。なければNotFound）
//!       ├ PlayerRegion  ：固定再生領域（準備中）
//!       └ AboutDialog   ：「Rustuneについて」
//! ```
//! 895px以下でのレイアウト切り替え（サイドバー→下部ナビゲーション）は、CSSのmedia queryだけで行う。Rust側に分岐はない。
//!
//! # 処理の流れ（画面遷移）
//! 1. `<A href=...>`をクリックすると、`leptos_router`がページを再読み込みせずにURLを変える（History API）。
//! 2. `use_location().pathname`（signal）が変わり、`Routes`が該当する画面に差し替える。
//! 3. 同じsignalに依存する`Effect`が再実行され、`title_for_path`で見出しを決め、`announce_page`でタイトルの更新とh1へのフォーカスを行う。
//!    h1の文字も同じ`page_title`クロージャを読んでいるので、自動で変わる。
//!
//! # ファイル
//! | ファイル | 内容 |
//! | --- | --- |
//! | `app_shell.rs` | `AppShell`（Router）と`Shell`（骨格の配置、見出しとフォーカスの更新） |
//! | `sidebar.rs` | `Sidebar`（ブランド・ナビゲーション・「Rustuneについて」ボタン） |
//! | `routes.rs` | `AppRoutes`（URLと画面の対応） |
//! | `nav.rs` | `NAV`と`title_for_path`（DOMを使わない定義と純粋関数、ユニットテスト付き） |
//! | `player_region.rs` | `PlayerRegion`（固定再生領域） |
//! | `about_dialog.rs` | `AboutDialog`（「Rustuneについて」） |
//! | `pages/` | 各routeの画面（1画面1ファイル） |
//!
//! このファイルにはモジュールの説明と`mod`宣言・`pub use`だけを置く。外へ公開するのは`AppShell`だけ。
//!
//! # Rustの読みどころ
//! - `mod sidebar;`は、同じディレクトリの`sidebar.rs`をサブモジュールとして読み込む宣言。
//! - `pub(crate)`は「このcrateの中からだけ使える」公開範囲。shellの部品はcrateの外（ギャラリーなど）へ出さない。
//! - `move || ...`は、使う変数をクロージャの中へ移して（所有権ごと取り込んで）作るクロージャ。
//!   leptosのsignalやNodeRefは`Copy`（コピーしても同じ値を指す軽い識別子）なので、複数のクロージャに渡せる。

mod about_dialog;
mod app_shell;
mod nav;
mod pages;
mod player_region;
mod routes;
mod sidebar;

pub use app_shell::AppShell;
