use crate::components::{Button, Dialog, EmptyState, ErrorNotice, TextField};
use leptos::{html, prelude::*};
use leptos_router::{components::*, hooks::use_location, path};
use rustune_contracts::HealthResponse;

const NAV: [(&str, &str, &str); 8] = [
    ("/library", "ライブラリ", "♫"),
    ("/albums", "アルバム", "▣"),
    ("/artists", "アーティスト", "♙"),
    ("/playlists", "プレイリスト", "≡"),
    ("/favorites", "お気に入り", "♡"),
    ("/imports", "取り込み", "↓"),
    ("/settings", "設定", "⚙"),
    ("/exclusions", "除外した音源", "⊘"),
];

#[component]
pub fn AppShell() -> impl IntoView {
    view! { <Router><Shell /></Router> }
}

#[component]
fn Shell() -> impl IntoView {
    let about_open = RwSignal::new(false);
    let location = use_location();
    let heading = NodeRef::<html::H1>::new();
    let page_title = move || title_for_path(&location.pathname.get());
    Effect::new(move |_| {
        let title = page_title();
        document().set_title(&format!("{title} — Rustune"));
        if let Some(element) = heading.get() {
            // 埋め込み表示（UIギャラリーのiframe）で親ページをスクロールさせない。
            let options = web_sys::FocusOptions::new();
            options.set_prevent_scroll(true);
            let _ = element.focus_with_options(&options);
        }
    });
    view! {
        <a class="skip-link" href="#main-content">"メインコンテンツへ"</a>
        <div class="app-shell">
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
            <main id="main-content" class="main-content" tabindex="-1">
                <header class="page-header"><div><p class="eyebrow">"YOUR MUSIC. YOUR SPACE."</p>
                    <h1 node_ref=heading tabindex="-1">{page_title}</h1></div><span class="stage-badge">"開発中"</span></header>
                <p class="stage-note">"画面の基盤を準備しています。楽曲の取り込み・管理・再生はまだ利用できません。"</p>
                <Routes fallback=NotFound>
                    <Route path=path!("") view=|| view! { <Redirect path="/library" /> } />
                    <Route path=path!("/library") view=Library />
                    <Route path=path!("/albums") view=|| view! { <Collection kind="アルバム" /> } />
                    <Route path=path!("/albums/:key") view=|| view! { <Collection kind="アルバム詳細" /> } />
                    <Route path=path!("/artists") view=|| view! { <Collection kind="アーティスト" /> } />
                    <Route path=path!("/artists/:key") view=|| view! { <Collection kind="アーティスト詳細" /> } />
                    <Route path=path!("/playlists") view=|| view! { <Collection kind="プレイリスト" /> } />
                    <Route path=path!("/playlists/:id") view=|| view! { <Collection kind="プレイリスト詳細" /> } />
                    <Route path=path!("/favorites") view=|| view! { <Collection kind="お気に入り" /> } />
                    <Route path=path!("/imports") view=Imports /><Route path=path!("/settings") view=Settings />
                    <Route path=path!("/exclusions") view=Exclusions />
                </Routes>
            </main>
            <PlayerRegion />
        </div>
        <Dialog open=about_open title="Rustuneについて">
            <p>"購入した音楽を自分のライブラリで管理する、開発中の音楽アプリです。"</p>
            <p><a href="https://www.apple.com/jp/itunes/" target="_blank" rel="noopener noreferrer">"iTunesの案内をブラウザで開く ↗"</a></p>
        </Dialog>
    }
}

/// 詳細画面（`/:key`・`/:id`）を持つセクション。`Routes`の定義と一致させる。
const DETAIL_SECTIONS: [&str; 3] = ["/albums", "/artists", "/playlists"];

/// `Routes`が実際に描画する画面に対応した見出しを返す。
/// 既知セクション配下でも未定義のパス（`/library/missing`など）は`NotFound`と同じ見出しにする。
/// ルーターは末尾スラッシュ1つを無視して照合するため（`/library/`はライブラリを描画）、同様に扱う。
fn title_for_path(path: &str) -> &'static str {
    let path = path.strip_suffix('/').unwrap_or(path);
    NAV.iter()
        .find(|(href, _, _)| {
            path == *href
                || (DETAIL_SECTIONS.contains(href)
                    && path
                        .strip_prefix(href)
                        .and_then(|rest| rest.strip_prefix('/'))
                        .is_some_and(|key| !key.is_empty() && !key.contains('/')))
        })
        .map(|(_, label, _)| *label)
        .unwrap_or(NOT_FOUND_TITLE)
}

const NOT_FOUND_TITLE: &str = "ページが見つかりません";

#[cfg(test)]
mod tests {
    use super::{NOT_FOUND_TITLE, title_for_path};

    #[test]
    fn known_collection_and_detail_paths_use_section_titles() {
        assert_eq!(title_for_path("/library"), "ライブラリ");
        assert_eq!(title_for_path("/library/"), "ライブラリ");
        assert_eq!(title_for_path("/albums/"), "アルバム");
        assert_eq!(title_for_path("/albums/key/"), "アルバム");
        assert_eq!(title_for_path("/albums"), "アルバム");
        assert_eq!(title_for_path("/albums/key"), "アルバム");
        assert_eq!(title_for_path("/artists/key"), "アーティスト");
        assert_eq!(title_for_path("/playlists/0f8e"), "プレイリスト");
        assert_eq!(title_for_path("/exclusions"), "除外した音源");
    }

    #[test]
    fn unsupported_paths_under_known_sections_are_not_found() {
        for path in [
            "/library/missing",
            "/albums/key/extra",
            "/albums//",
            "/favorites/x",
            "/settings/advanced",
            "/librar",
            "/libraryx",
            "/unknown",
        ] {
            assert_eq!(title_for_path(path), NOT_FOUND_TITLE, "{path}");
        }
    }
}

#[component]
fn Library() -> impl IntoView {
    view! {
        <TextField id="library-search" label="ライブラリ内を検索" placeholder="曲・アルバム・アーティスト（準備中）" disabled=true />
        <EmptyState title="あなたの音楽を集めよう" description="取り込み設定から音源フォルダを指定すると、ここに楽曲が並びます。">
            <A href="/settings" attr:class="button-link">"取り込み設定へ"</A>
            <A href="/imports" attr:class="text-link">"取り込み状況を見る"</A>
        </EmptyState>
    }
}

#[component]
fn Collection(kind: &'static str) -> impl IntoView {
    view! { <EmptyState title="音楽がまだありません" description="ライブラリへの取り込み機能は準備中です。">
        <p class="muted">{format!("{kind}の表示は、楽曲を取り込んだあとに利用できます。")}</p>
        <A href="/library" attr:class="button-link secondary">"ライブラリへ"</A>
    </EmptyState> }
}

#[component]
fn Imports() -> impl IntoView {
    view! { <EmptyState title="取り込み履歴はまだありません" description="フォルダの走査と取り込み処理は準備中です。">
        <Button disabled=true>"スキャンを開始（準備中）"</Button>
        <A href="/settings" attr:class="text-link">"取り込み設定へ"</A>
    </EmptyState> }
}

#[component]
fn Exclusions() -> impl IntoView {
    view! { <EmptyState title="除外した音源はありません" description="取り込みから除外した音源の管理機能は準備中です。">
        <A href="/library" attr:class="button-link secondary">"ライブラリへ"</A>
    </EmptyState> }
}

#[component]
fn Settings() -> impl IntoView {
    let checking = RwSignal::new(false);
    let status = RwSignal::new("接続を確認してください".to_string());
    let error = RwSignal::new(None::<String>);
    let check = Callback::new(move |_| {
        if checking.get_untracked() {
            return;
        }
        checking.set(true);
        error.set(None);
        status.set("接続中…".into());
        leptos::task::spawn_local(async move {
            let result = async {
                let response = gloo_net::http::Request::get("/api/v1/health")
                    .send()
                    .await?;
                if !response.ok() {
                    return Err(gloo_net::Error::GlooError("unavailable".into()));
                }
                response.json::<HealthResponse>().await
            }
            .await;
            match result {
                Ok(health) if health.service == "rustune" => {
                    status.set("Rustuneサーバーに接続しました".into())
                }
                _ => {
                    status.set("接続できませんでした".into());
                    error.set(Some(
                        "サーバーの起動を確認して、もう一度お試しください。".into(),
                    ));
                }
            }
            checking.set(false);
        });
    });
    view! {
        <section class="settings-card"><h2>"取り込み設定"</h2><p class="muted">"音源フォルダと自動取り込みの設定は準備中です。"</p>
            <A href="/imports" attr:class="text-link">"取り込み状況へ"</A></section>
        <section class="settings-card"><h2>"サーバー接続"</h2><p role="status" aria-live="polite">{move || status.get()}</p>
            <button type="button" disabled=move || checking.get() on:click=move |_| check.run(())>
                {move || if checking.get() { "確認中…" } else { "接続を確認" }}
            </button>
            {move || error.get().map(|message| view! { <ErrorNotice message=message /> })}
        </section>
    }
}

#[component]
fn PlayerRegion() -> impl IntoView {
    view! { <section class="player-region" aria-label="再生コントロール（準備中）">
        <div class="track-placeholder"><div class="cover-placeholder" aria-hidden="true">"♫"</div>
            <div><p>"再生する曲を選んでください"</p><span>"プレイヤーは準備中です"</span></div></div>
        <div class="player-controls">
            <button type="button" class="icon-button" aria-label="前の曲（準備中）" disabled>"⏮"</button>
            <button type="button" class="icon-button play-button" aria-label="再生（準備中）" disabled>"▶"</button>
            <button type="button" class="icon-button" aria-label="次の曲（準備中）" disabled>"⏭"</button>
        </div><span class="player-time" aria-label="再生時間未設定">"—:— / —:—"</span>
    </section> }
}

#[component]
fn NotFound() -> impl IntoView {
    view! { <EmptyState title=NOT_FOUND_TITLE description="ナビゲーションから移動先を選んでください。">
        <A href="/library" attr:class="button-link">"ライブラリへ"</A>
    </EmptyState> }
}
