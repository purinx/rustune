use leptos::prelude::*;

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
