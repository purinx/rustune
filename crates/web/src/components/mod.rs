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

use leptos::{html, prelude::*};
use web_sys::wasm_bindgen::JsCast;

/// ボタン。既定は緑の主操作、`secondary=true`で暗色の副操作になる。
///
/// `type="button"`を固定し、form内に置いても送信ボタンとして動かないようにする。
/// `on_click`を省略したときは、クリックしても何もしない（`if let Some(...)`で、`Some`のときだけ呼ぶ）。
#[component]
pub fn Button(
    children: Children,
    #[prop(default = false)] secondary: bool,
    #[prop(default = false)] disabled: bool,
    #[prop(optional)] on_click: Option<Callback<()>>,
) -> impl IntoView {
    view! {
        <button type="button" class:secondary=secondary disabled=disabled
            on:click=move |_| { if let Some(callback) = on_click { callback.run(()); } }>{children()}</button>
    }
}

/// ラベル付きの検索入力欄。`label`の`for`と`input`の`id`を一致させ、ラベルのクリックで入力欄へ移れるようにする。
/// 入力値の保持や検索処理はまだ持たない（ライブラリ画面の検索はタスク13で接続する）。
#[component]
pub fn TextField(
    id: &'static str,
    label: &'static str,
    placeholder: &'static str,
    #[prop(default = false)] disabled: bool,
) -> impl IntoView {
    view! { <div class="field"><label for=id>{label}</label>
    <input id=id type="search" placeholder=placeholder disabled=disabled /></div> }
}

/// 表示するデータがないときの案内。見出し・説明と、次の操作（`children`のリンクやボタン）を並べる。
/// `aria-hidden="true"`の音符は装飾なので、スクリーンリーダーには読ませない。
#[component]
pub fn EmptyState(
    title: &'static str,
    description: &'static str,
    children: Children,
) -> impl IntoView {
    view! { <section class="empty-state"><div class="empty-art" aria-hidden="true">"♫"</div>
    <h2>{title}</h2><p>{description}</p><div class="actions">{children()}</div></section> }
}

/// エラーメッセージ。`role="alert"`により、表示された時点でスクリーンリーダーが読み上げる。
/// `message`は`String`（所有する文字列）で受け取る。実行時に作った文言を渡せるようにするため。
#[component]
pub fn ErrorNotice(message: String) -> impl IntoView {
    view! { <p class="error-notice" role="alert">{message}</p> }
}

/// モーダルダイアログ。開閉の状態は、親が持つ`open`シグナルで制御する。
///
/// # 開閉
/// - `open`が`true`になると、`Effect`がnativeの`showModal()`を呼ぶ。`false`になると`close()`を呼ぶ。
///   `Effect::new`は、中で読んだsignal（ここでは`open`）が変わるたびに再実行される副作用。
/// - `showModal()`を使うと、次の動作はブラウザが行う。
///   背景のinert化（背景をクリックもフォーカスもできなくする）、Escapeで閉じる、閉じた後に開く前の要素へフォーカスを戻す。
/// - Escapeや「閉じる」でブラウザ側から閉じた場合も、`close`・`cancel`イベントで`open`を`false`に戻す。
///   これで、Rust側の状態とDOMの状態がずれないようにする。
///
/// # Tabキーの循環
/// ダイアログ内のフォーカスできる要素を集め、最後の要素でTabなら最初へ、最初の要素でShift+Tabなら最後へ移す。
/// ブラウザ標準のままだと、アドレスバーなどへフォーカスが抜けることがあるため、明示的に循環させる。
///
/// # Rustの読みどころ
/// - `NodeRef`は、描画後のDOM要素への参照。描画前に`get()`すると`None`を返す。
/// - `let Some(x) = ... else { return; };`は、`None`なら早期returnし、`Some`なら中身を`x`に取り出す書き方。
/// - `dyn_into::<HtmlElement>()`は、JSの汎用的な`Node`を、`focus()`を持つ`HtmlElement`型へ変換する（失敗時は`Err`）。
/// - `let _ = ...;`は、戻り値（ここでは失敗し得るDOM操作の`Result`）を意図的に無視する書き方。
#[component]
pub fn Dialog(open: RwSignal<bool>, title: &'static str, children: Children) -> impl IntoView {
    let dialog = NodeRef::<html::Dialog>::new();
    Effect::new(move |_| {
        let visible = open.get();
        if let Some(element) = dialog.get() {
            if visible && !element.open() {
                let _ = element.show_modal();
            } else if !visible && element.open() {
                element.close();
            }
        }
    });
    view! {
        <dialog node_ref=dialog aria-labelledby="shell-dialog-title"
            on:close=move |_| open.set(false) on:cancel=move |_: web_sys::Event| open.set(false)
            on:keydown=move |event: web_sys::KeyboardEvent| {
                if event.key() != "Tab" { return; }
                let Some(element) = dialog.get() else { return; };
                let Ok(items) = element.query_selector_all("a[href], button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex='-1'])") else { return; };
                if items.length() == 0 { event.prevent_default(); return; }
                let first = items.item(0).and_then(|node| node.dyn_into::<web_sys::HtmlElement>().ok());
                let last = items.item(items.length() - 1).and_then(|node| node.dyn_into::<web_sys::HtmlElement>().ok());
                let active = document().active_element();
                if let (Some(first), Some(last), Some(active)) = (first, last, active) {
                    if event.shift_key() && active == *first { event.prevent_default(); let _ = last.focus(); }
                    else if !event.shift_key() && active == *last { event.prevent_default(); let _ = first.focus(); }
                }
            }>
            <h2 id="shell-dialog-title">{title}</h2>{children()}
            <Button on_click=Callback::new(move |_| open.set(false))>"閉じる"</Button>
        </dialog>
    }
}
