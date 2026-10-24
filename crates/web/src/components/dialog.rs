use super::Button;
use leptos::{html, prelude::*};
use web_sys::{HtmlDialogElement, HtmlElement, KeyboardEvent, wasm_bindgen::JsCast};

/// ダイアログ内でフォーカスできる要素を選ぶCSSセレクタ。無効な入力欄と`tabindex="-1"`は除く。
const FOCUSABLE: &str = "a[href], button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex='-1'])";

/// モーダルダイアログ。開閉の状態は、親が持つ`open`シグナルで制御する。
///
/// # 開閉
/// - `open`が`true`になると、`Effect`が[`sync_open_state`]を通じてnativeの`showModal()`を呼ぶ。`false`になると`close()`を呼ぶ。
///   `Effect::new`は、中で読んだsignal（ここでは`open`）が変わるたびに再実行される副作用。
/// - `showModal()`を使うと、次の動作はブラウザが行う。
///   背景のinert化（背景をクリックもフォーカスもできなくする）、Escapeで閉じる、閉じた後に開く前の要素へフォーカスを戻す。
/// - Escapeや「閉じる」でブラウザ側から閉じた場合も、`close`・`cancel`イベントで`open`を`false`に戻す。
///   これで、Rust側の状態とDOMの状態がずれないようにする。
///
/// # Tabキーの循環
/// Tabキーの処理は[`trap_tab_focus`]に分けている。`view!`の中では、イベントをその関数へ渡すだけにする。
///
/// # Rustの読みどころ
/// - `NodeRef`は、描画後のDOM要素への参照。描画前に`get()`すると`None`を返す。
#[component]
pub fn Dialog(open: RwSignal<bool>, title: &'static str, children: Children) -> impl IntoView {
    let dialog = NodeRef::<html::Dialog>::new();
    Effect::new(move |_| sync_open_state(open.get(), dialog));
    view! {
        <dialog node_ref=dialog aria-labelledby="shell-dialog-title"
            on:close=move |_| open.set(false) on:cancel=move |_: web_sys::Event| open.set(false)
            on:keydown=move |event: KeyboardEvent| trap_tab_focus(&event, dialog)>
            <h2 id="shell-dialog-title">{title}</h2>{children()}
            <Button on_click=Callback::new(move |_| open.set(false))>"閉じる"</Button>
        </dialog>
    }
}

/// `open`の値に合わせて、nativeの`<dialog>`を開く（`showModal()`）か閉じる（`close()`）。
/// 描画前で要素がまだないとき（`dialog.get()`が`None`）と、すでに同じ状態のときは何もしない。
fn sync_open_state(visible: bool, dialog: NodeRef<html::Dialog>) {
    let Some(element) = dialog.get() else { return };
    if visible && !element.open() {
        let _ = element.show_modal();
    } else if !visible && element.open() {
        element.close();
    }
}

/// Tabキーによるフォーカス移動を、ダイアログの中で循環させる（`keydown`のハンドラー）。
///
/// 最後の要素でTabなら最初の要素へ、最初の要素でShift+Tabなら最後の要素へ移す。
/// それ以外の位置では何もせず、ブラウザ標準の移動に任せる。
/// フォーカスできる要素が1つもなければ、Tabを打ち消して、フォーカスがダイアログの外へ出ないようにする。
/// ブラウザ標準のままだと、アドレスバーなどへフォーカスが抜けることがあるため、明示的に循環させる。
///
/// `let Some(x) = ... else { return };`は、`None`なら早期returnし、`Some`なら中身を`x`に取り出す書き方。
/// `let _ = ...;`は、戻り値（ここでは失敗し得る`focus()`の`Result`）を意図的に無視する書き方。
fn trap_tab_focus(event: &KeyboardEvent, dialog: NodeRef<html::Dialog>) {
    if event.key() != "Tab" {
        return;
    }
    let Some(element) = dialog.get() else { return };
    let Some((first, last)) = focusable_edges(&element) else {
        event.prevent_default();
        return;
    };
    let Some(active) = document().active_element() else {
        return;
    };
    let target = if event.shift_key() && active == *first {
        last
    } else if !event.shift_key() && active == *last {
        first
    } else {
        return;
    };
    event.prevent_default();
    let _ = target.focus();
}

/// ダイアログ内でフォーカスできる、最初と最後の要素を返す。1つもなければ`None`。
///
/// `query_selector_all`は汎用的な`Node`の一覧を返すので、`dyn_into::<HtmlElement>()`で
/// `focus()`を持つ`HtmlElement`型へ変換する（変換できなければ`None`）。
/// `?`は、`None`（または`Err`）ならその場で`None`を返す演算子。
fn focusable_edges(dialog: &HtmlDialogElement) -> Option<(HtmlElement, HtmlElement)> {
    let items = dialog.query_selector_all(FOCUSABLE).ok()?;
    let item_at = |index| {
        items
            .item(index)
            .and_then(|node| node.dyn_into::<HtmlElement>().ok())
    };
    Some((item_at(0)?, item_at(items.length().checked_sub(1)?)?))
}
