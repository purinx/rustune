use super::{
    about_dialog::AboutDialog, nav::title_for_path, player_region::PlayerRegion, routes::AppRoutes,
    sidebar::Sidebar,
};
use leptos::{html, prelude::*};
use leptos_router::{components::Router, hooks::use_location};

/// アプリのルートコンポーネント。`Router`で包み、配下で`use_location`や`<A>`が使えるようにする。
#[component]
pub fn AppShell() -> impl IntoView {
    view! { <Router><Shell /></Router> }
}

/// 画面遷移しても残る骨格を描画する。各状態の役割は次のとおり。
/// - `about_open`：「Rustuneについて」ダイアログの開閉。[`Sidebar`]のボタンと[`AboutDialog`]が共有する。
/// - `location`：現在のURL。`pathname`はsignalで、URLが変わると依存箇所が再実行される。
/// - `heading`：h1要素への参照。画面遷移時にフォーカスを移すために使う。
/// - `page_title`：URLから見出しを計算するクロージャ。h1の表示と`Effect`の両方が読む。
#[component]
fn Shell() -> impl IntoView {
    let about_open = RwSignal::new(false);
    let location = use_location();
    let heading = NodeRef::<html::H1>::new();
    let page_title = move || title_for_path(&location.pathname.get());
    Effect::new(move |_| announce_page(page_title(), heading));
    view! {
        <a class="skip-link" href="#main-content">"メインコンテンツへ"</a>
        <div class="app-shell">
            <Sidebar about_open=about_open />
            <main id="main-content" class="main-content" tabindex="-1">
                <header class="page-header"><div><p class="eyebrow">"YOUR MUSIC. YOUR SPACE."</p>
                    <h1 node_ref=heading tabindex="-1">{page_title}</h1></div><span class="stage-badge">"開発中"</span></header>
                <p class="stage-note">"画面の基盤を準備しています。楽曲の取り込み・管理・再生はまだ利用できません。"</p>
                <AppRoutes />
            </main>
            <PlayerRegion />
        </div>
        <AboutDialog open=about_open />
    }
}

/// 画面が変わったことを伝える：ウィンドウタイトルを「<見出し> — Rustune」にし、h1へフォーカスを移す。
/// スクリーンリーダーは、フォーカスが移ったh1を読み上げる。
fn announce_page(title: &str, heading: NodeRef<html::H1>) {
    document().set_title(&format!("{title} — Rustune"));
    if let Some(element) = heading.get() {
        // 埋め込み表示（UIギャラリーのiframe）で親ページをスクロールさせない。
        let options = web_sys::FocusOptions::new();
        options.set_prevent_scroll(true);
        let _ = element.focus_with_options(&options);
    }
}
