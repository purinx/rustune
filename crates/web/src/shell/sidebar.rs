use super::nav::NAV;
use crate::components::Button;
use leptos::prelude::*;
use leptos_router::components::A;

/// サイドバー（895px以下では下部ナビゲーション）。ブランド、[`NAV`]のリンク、「Rustuneについて」ボタンを並べる。
///
/// `about_open`は「Rustuneについて」ダイアログの開閉状態。ボタンを押すと`true`にする。
/// 幅による配置の切り替えは`style.css`のmedia queryだけで行い、ここでは分岐しない。
#[component]
pub(crate) fn Sidebar(about_open: RwSignal<bool>) -> impl IntoView {
    view! {
        <aside class="sidebar">
            <A href="/library" attr:class="brand"><img src="/icon.svg" alt="" width="36" height="36" />"Rustune"</A>
            <p class="nav-caption">"あなたの音楽"</p>
            <nav aria-label="メインナビゲーション" class="primary-nav">
                {NAV.into_iter().map(|(href, label, icon)| view! {
                    <A href=href attr:class="nav-link"><span aria-hidden="true" class="nav-icon">{icon}</span><span>{label}</span></A>
                }).collect_view()}
            </nav>
            <div class="sidebar-footer">
                <Button secondary=true on_click=Callback::new(move |_| about_open.set(true))>"Rustuneについて"</Button>
                <p>"好きな音楽を、あなたの場所に。"</p>
            </div>
        </aside>
    }
}
