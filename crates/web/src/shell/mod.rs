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
//!       ├ aside.sidebar ：ブランド、NAVのリンク、「Rustuneについて」ボタン
//!       ├ main          ：見出し（h1）＋ Routes（URLに一致する画面を1つ描画。なければNotFound）
//!       ├ PlayerRegion  ：固定再生領域（準備中）
//!       └ Dialog        ：「Rustuneについて」
//! ```
//! 895px以下でのレイアウト切り替え（サイドバー→下部ナビゲーション）は、CSSのmedia queryだけで行う。Rust側に分岐はない。
//!
//! # 処理の流れ（画面遷移）
//! 1. `<A href=...>`をクリックすると、`leptos_router`がページを再読み込みせずにURLを変える（History API）。
//! 2. `use_location().pathname`（signal）が変わり、`Routes`が該当する画面に差し替える。
//! 3. 同じsignalに依存する`Effect`が再実行され、`title_for_path`で見出しを決め、タイトルの更新とh1へのフォーカスを行う。
//!    h1の文字も同じ`page_title`クロージャを読んでいるので、自動で変わる。
//!
//! # Rustの読みどころ
//! - `const NAV: [(&str, &str, &str); 8]`は、(パス, 表示名, アイコン)の組を8個持つ固定長の配列。
//!   `&str`は文字列への参照で、文字列リテラルはプログラム全体の寿命（`'static`）を持つ。
//! - `move || ...`は、使う変数をクロージャの中へ移して（所有権ごと取り込んで）作るクロージャ。
//!   leptosのsignalやNodeRefは`Copy`（コピーしても同じ値を指す軽い識別子）なので、複数のクロージャに渡せる。
//! - routeの`view=Library`のようにコンポーネント名を渡す書き方と、`view=|| view! { ... }`のように
//!   引数なしのクロージャを渡す書き方がある。後者は、propsを渡して描画したいときに使う。

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

/// アプリのルートコンポーネント。`Router`で包み、配下で`use_location`や`<A>`が使えるようにする。
#[component]
pub fn AppShell() -> impl IntoView {
    view! { <Router><Shell /></Router> }
}

/// 画面遷移しても残る骨格を描画する。各状態の役割は次のとおり。
/// - `about_open`：「Rustuneについて」ダイアログの開閉。
/// - `location`：現在のURL。`pathname`はsignalで、URLが変わると依存箇所が再実行される。
/// - `heading`：h1要素への参照。画面遷移時にフォーカスを移すために使う。
/// - `page_title`：URLから見出しを計算するクロージャ。h1の表示と`Effect`の両方が読む。
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
    // 末尾の`/`を1つだけ取り除く。`strip_suffix`は、一致しなければ`None`を返すので、そのときは元の値を使う。
    let path = path.strip_suffix('/').unwrap_or(path);
    // NAVを先頭から調べ、条件に合う最初の項目の表示名を返す。見つからなければNOT_FOUND_TITLE。
    // 条件は「一覧のパスと完全一致」か、「詳細を持つセクションの直下に、空でない1階層だけが続く」こと。
    // `strip_prefix`は接頭辞を除いた残りを`Option`で返し、`and_then`は`Some`のときだけ次の処理へ進む。
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

/// ライブラリ画面（準備中）。検索欄は無効状態で表示し、取り込み設定への導線を出す。
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

/// アルバム・アーティスト・プレイリスト・お気に入りと、その詳細画面に共通の準備中表示。
/// `kind`で画面名を差し替える。詳細画面の`:key`・`:id`は、まだ読み取らない。
#[component]
fn Collection(kind: &'static str) -> impl IntoView {
    view! { <EmptyState title="音楽がまだありません" description="ライブラリへの取り込み機能は準備中です。">
        <p class="muted">{format!("{kind}の表示は、楽曲を取り込んだあとに利用できます。")}</p>
        <A href="/library" attr:class="button-link secondary">"ライブラリへ"</A>
    </EmptyState> }
}

/// 取り込み画面（準備中）。スキャン開始ボタンは無効状態で表示する。
#[component]
fn Imports() -> impl IntoView {
    view! { <EmptyState title="取り込み履歴はまだありません" description="フォルダの走査と取り込み処理は準備中です。">
        <Button disabled=true>"スキャンを開始（準備中）"</Button>
        <A href="/settings" attr:class="text-link">"取り込み設定へ"</A>
    </EmptyState> }
}

/// 除外した音源の画面（準備中）。
#[component]
fn Exclusions() -> impl IntoView {
    view! { <EmptyState title="除外した音源はありません" description="取り込みから除外した音源の管理機能は準備中です。">
        <A href="/library" attr:class="button-link secondary">"ライブラリへ"</A>
    </EmptyState> }
}

/// 設定画面。今の段階で、実際にサーバーと通信する唯一の画面。
///
/// 「接続を確認」を押すと`GET /api/v1/health`を呼び、応答の`service`が`"rustune"`なら接続成功と表示する。
/// - `checking`：通信中のフラグ。通信中はボタンを無効にして、二重送信を防ぐ。
///   `get_untracked()`は、依存を登録せずに値だけを読む（クリック処理の中なので再実行は不要）。
/// - `status`：結果の文言。`role="status"`と`aria-live="polite"`の領域に出すので、スクリーンリーダーが読み上げる。
/// - `error`：失敗したときだけ`Some`になり、`ErrorNotice`を表示する。`None`のときは何も描画しない。
///
/// 非同期処理について：`spawn_local`は、ブラウザのイベントループ上で`async`ブロックを実行する。
/// WASMはスレッドを使わないため、`await`の間は他の処理に制御を返す。
/// 内側の`async { ... }.await`の中では`?`演算子を使い、通信・HTTPステータス・JSON解析のどこで失敗しても
/// 1つの`Err`にまとめてから、`match`で表示を決める。
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

/// 画面下部に固定する再生領域（準備中）。画面遷移しても消えないよう、`Routes`の外に置く。
/// ボタンは無効状態で、`aria-label`に「準備中」を含める。実際の再生処理はタスク12で接続する。
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

/// どのrouteにも一致しないURLで表示する画面。見出しは`title_for_path`と同じ`NOT_FOUND_TITLE`を使う。
#[component]
fn NotFound() -> impl IntoView {
    view! { <EmptyState title=NOT_FOUND_TITLE description="ナビゲーションから移動先を選んでください。">
        <A href="/library" attr:class="button-link">"ライブラリへ"</A>
    </EmptyState> }
}
