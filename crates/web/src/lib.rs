//! Rustune Web UI（WASM）の画面と共通部品。
//!
//! # 責務
//! - 画面遷移と各画面の骨格（[`shell`]）、共通部品（[`components`]）を公開する。
//! - アプリ本体の起動（`<body>`へのmount）は`main.rs`が行う。ここには起動処理を書かない。
//!
//! # libとbinに分けている理由
//! 開発専用のUIギャラリー（`crates/stories`の`rustune-stories`）が、同じ部品と画面を描画するため。
//! Rustでは、binary（`main.rs`）の中のモジュールは他のcrateから参照できない。
//! そこで部品と画面をlibrary（このファイル）に置き、アプリ本体の`main.rs`とギャラリーの両方がここを使う。
//! こうすることで、ギャラリーのコードはアプリ本体の成果物に入らない。
//!
//! ```mermaid
//! flowchart LR
//!     bin["rustune-web（bin：main.rs）"] --> lib["rustune-web（lib：shell・components）"]
//!     stories["rustune-stories（開発専用）"] --> lib
//!     lib --> contracts["rustune-contracts"]
//!     lib --> domain["rustune-domain"]
//! ```

pub mod components;
pub mod shell;
