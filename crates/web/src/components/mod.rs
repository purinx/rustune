//! 画面をまたいで使う共通UI部品。
//!
//! # 責務
//! - ボタン・入力欄・空状態・エラー表示・ダイアログの構造と基本動作を1か所にまとめる。
//!   部品はclass名を付けるだけで、色・角丸・余白は`style.css`の共有トークンが決める（AGENTS.md）。
//! - アクセシビリティ上の決まりを部品側で保証し、各画面で書き忘れないようにする。
//!   例：ボタンの`type="button"`、エラーの`role="alert"`、ダイアログのフォーカス管理。
//!
//! # 責務外
//! API通信や業務状態は持たない。表示内容とイベントの通知先（`Callback`）は呼び出し側が渡す。
//!
//! # ファイル
//! 部品ごとに1ファイル（`button.rs`、`text_field.rs`、`empty_state.rs`、`error_notice.rs`、`dialog.rs`）。
//! このファイルにはモジュールの説明と`pub use`だけを置き、外からは`crate::components::Button`のように使う。
//!
//! # leptosの読みどころ
//! - `#[component]`を付けた関数がコンポーネント。引数がprops（`<Button secondary=true>`の属性）になる。
//!   戻り値の`impl IntoView`は「DOMに変換できる何かの型」を表す。
//! - コンポーネント関数は最初に1回だけ実行される。その後は、signalを読んでいる箇所だけが再実行される。
//!   Reactのように、関数全体が毎回走るわけではない。
//! - `view! { ... }`は、HTMLに近い記法でDOMを組み立てるマクロ。`"文字列"`はテキスト、`{式}`は値の埋め込み。
//! - `#[prop(default = false)]`は省略できるprop。`#[prop(optional)]`は、省略時に`None`になるprop。
//! - `Children`は子要素（`<Button>"ここ"</Button>`の中身）。`children()`で描画する。
//! - `Callback<()>`は親から渡すイベントハンドラ。`run(())`で呼ぶ。`()`は「値なし」を表す型。
//! - `RwSignal<T>`は読み書きできるリアクティブな値。`get()`で読むと、読んだ箇所が依存として登録される。
//!   `set()`で値が変わると、依存している箇所だけが再実行される。

mod button;
mod dialog;
mod empty_state;
mod error_notice;
mod text_field;

pub use button::Button;
pub use dialog::Dialog;
pub use empty_state::EmptyState;
pub use error_notice::ErrorNotice;
pub use text_field::TextField;
