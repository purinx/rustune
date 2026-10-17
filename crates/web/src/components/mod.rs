use leptos::{html, prelude::*};
use web_sys::wasm_bindgen::JsCast;

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

#[component]
pub fn EmptyState(
    title: &'static str,
    description: &'static str,
    children: Children,
) -> impl IntoView {
    view! { <section class="empty-state"><div class="empty-art" aria-hidden="true">"♫"</div>
    <h2>{title}</h2><p>{description}</p><div class="actions">{children()}</div></section> }
}

#[component]
pub fn ErrorNotice(message: String) -> impl IntoView {
    view! { <p class="error-notice" role="alert">{message}</p> }
}

/// Escape処理・背景のinert化・フォーカスの閉じ込めと復帰はnativeのmodalに任せる。
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
