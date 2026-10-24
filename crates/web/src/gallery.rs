//! `trunk serve`の`/gallery`で表示する、開発専用のUIギャラリー（Storybookの代わり）。
//!
//! # 責務
//! - 共有CSSトークン（配色・文字・角丸・影）の見本を並べ、`design.md`との整合を目で確認できるようにする。
//! - 共通部品（[`crate::components`]）を、状態（通常・副操作・無効など）ごとに並べる。
//! - 実際の各画面をiframeで読み込み、390pxと1280pxの幅で同時に確認できるようにする。
//!
//! # 責務外
//! 製品の機能ではない。`main.rs`の`#[cfg(debug_assertions)]`により、debugビルドのときだけコンパイルされ、
//! releaseの成果物には含まれない。ギャラリー専用のCSSは`GALLERY_CSS`に閉じ込め、`style.css`へは足さない。
//!
//! # 仕組み
//! - 見本の一覧は、`COLORS`などの定数配列で持つ。`into_iter().map(...).collect_view()`で配列から要素を並べる。
//! - 画面プレビューのiframeは、同じ開発サーバーのURL（例：`/library`）を読み込むので、アプリ本体がそのまま動く。
//!   1280px側は、幅1280pxのiframeを`transform: scale(.5)`で半分に縮めて表示している。
//!   iframeの幅を基準にmedia queryが効くので、390px側では下部ナビゲーションのレイアウトになる。
//! - 部品を追加・変更したら、ここにも見本を追加する（README参照）。

use crate::components::{Button, Dialog, EmptyState, ErrorNotice, TextField};
use leptos::prelude::*;

const COLORS: [(&str, &str); 11] = [
    ("--background", "Near Black"),
    ("--surface", "Dark Surface"),
    ("--surface-interactive", "Mid Dark"),
    ("--surface-elevated", "Dark Card"),
    ("--text", "White"),
    ("--text-secondary", "Silver"),
    ("--accent", "Accent"),
    ("--on-accent", "On Accent"),
    ("--negative", "Negative Red"),
    ("--border", "Light Border"),
    ("--overlay", "Overlay"),
];

const TYPE_SCALE: [(&str, &str, &str); 6] = [
    ("--text-title", "--weight-bold", "Section Title 24px"),
    ("--text-feature", "--weight-bold", "Feature Heading 18px"),
    ("--text-body", "--weight-regular", "Body 16px"),
    ("--text-caption", "--weight-bold", "Caption Bold 14px"),
    ("--text-caption", "--weight-regular", "Caption 14px"),
    ("--text-small", "--weight-regular", "Small 12px"),
];

const RADII: [&str; 4] = [
    "--radius-art",
    "--radius-card",
    "--radius-pill",
    "--radius-circle",
];
const SHADOWS: [&str; 3] = ["--shadow-medium", "--shadow-heavy", "--shadow-input"];

const SCREENS: [(&str, &str); 10] = [
    ("/library", "ライブラリ"),
    ("/albums", "アルバム"),
    ("/artists", "アーティスト"),
    ("/playlists", "プレイリスト"),
    ("/favorites", "お気に入り"),
    ("/imports", "取り込み"),
    ("/settings", "設定"),
    ("/exclusions", "除外した音源"),
    ("/albums/sample", "詳細URL"),
    ("/unknown", "未知URL"),
];

const GALLERY_CSS: &str = r#"
.gallery { max-width: 1440px; margin: 0 auto; padding: var(--space-8) var(--space-4); }
.gallery > header p { color: var(--text-secondary); font-size: var(--text-caption); }
.gallery-section { margin-top: var(--space-8); padding: var(--space-6); background: var(--surface); border-radius: var(--radius-card); }
.gallery-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(220px, 1fr)); gap: var(--space-4); }
.gallery-case { padding: var(--space-4); background: var(--background); border-radius: var(--radius-card); }
.gallery-case > h3 { margin: 0 0 var(--space-3); color: var(--text-secondary); font-size: var(--text-small); font-weight: var(--weight-bold); }
.gallery-row { display: flex; flex-wrap: wrap; align-items: center; gap: var(--space-3); }
.gallery-swatch { height: 56px; margin-bottom: var(--space-2); border-radius: var(--radius-art); box-shadow: var(--shadow-input); }
.gallery-token { color: var(--text-secondary); font-size: var(--text-small); }
.gallery-shape { width: 72px; height: 48px; background: var(--surface-interactive); }
.gallery-elevation { height: 64px; background: var(--surface-elevated); border-radius: var(--radius-card); }
.gallery-tabs { display: flex; flex-wrap: wrap; gap: var(--space-2); margin-bottom: var(--space-5); }
.gallery-tabs button { min-height: 36px; padding: var(--space-2) var(--space-4); letter-spacing: 0; text-transform: none; }
.gallery-frames { display: flex; flex-wrap: wrap; align-items: flex-start; gap: var(--space-6); }
.gallery-frames figure { margin: 0; }
.gallery-frames figcaption { margin-bottom: var(--space-2); color: var(--text-secondary); font-size: var(--text-small); }
.gallery-frame { border: 0; border-radius: var(--radius-card); background: var(--background); box-shadow: var(--shadow-heavy); }
.gallery-scaled { width: 640px; height: 420px; overflow: hidden; border-radius: var(--radius-card); }
.gallery-scaled iframe { width: 1280px; height: 840px; transform: scale(.5); transform-origin: 0 0; }
.gallery-nav { position: static; inset: auto; height: auto; padding: 0; background: none; }
"#;

#[component]
pub fn Gallery() -> impl IntoView {
    document().set_title("UIギャラリー — Rustune");
    view! {
        <style>{GALLERY_CSS}</style>
        <div class="gallery">
            <header>
                <h1>"Rustune UIギャラリー"</h1>
                <p>"開発ビルド専用の確認ページです。共有CSSトークンと共通部品を、状態ごとに並べています。"</p>
            </header>
            <Tokens />
            <Components />
            <Screens />
        </div>
    }
}

fn css_var(name: &str) -> String {
    format!("var({name})")
}

#[component]
fn Tokens() -> impl IntoView {
    view! {
        <section class="gallery-section">
            <h2>"配色"</h2>
            <div class="gallery-grid">
                {COLORS.into_iter().map(|(token, label)| view! {
                    <div class="gallery-case">
                        <div class="gallery-swatch" style:background=css_var(token)></div>
                        <strong>{label}</strong><div class="gallery-token">{token}</div>
                    </div>
                }).collect_view()}
            </div>
        </section>
        <section class="gallery-section">
            <h2>"タイポグラフィ"</h2>
            {TYPE_SCALE.into_iter().map(|(size, weight, label)| view! {
                <p style:font-size=css_var(size) style:font-weight=css_var(weight)>
                    {label}" — あなたの音楽を集めよう Rustune"
                    <span class="gallery-token">{format!("  {size} / {weight}")}</span>
                </p>
            }).collect_view()}
        </section>
        <section class="gallery-section">
            <h2>"角丸・影"</h2>
            <div class="gallery-grid">
                {RADII.into_iter().map(|token| view! {
                    <div class="gallery-case"><div class="gallery-shape" style:border-radius=css_var(token)></div>
                    <div class="gallery-token">{token}</div></div>
                }).collect_view()}
                {SHADOWS.into_iter().map(|token| view! {
                    <div class="gallery-case"><div class="gallery-elevation" style:box-shadow=css_var(token)></div>
                    <div class="gallery-token">{token}</div></div>
                }).collect_view()}
            </div>
        </section>
    }
}

#[component]
fn Components() -> impl IntoView {
    let dialog_open = RwSignal::new(false);
    view! {
        <section class="gallery-section">
            <h2>"共通部品"</h2>
            <div class="gallery-grid">
                <div class="gallery-case"><h3>"Button"</h3><div class="gallery-row">
                    <Button>"主操作"</Button>
                    <Button secondary=true>"副操作"</Button>
                    <Button disabled=true>"無効"</Button>
                    <Button secondary=true disabled=true>"副・無効"</Button>
                </div></div>
                <div class="gallery-case"><h3>"リンク"</h3><div class="gallery-row">
                    <a class="button-link" href="#">"ボタン型リンク"</a>
                    <a class="button-link secondary" href="#">"副リンク"</a>
                    <a class="text-link" href="#">"テキストリンク"</a>
                </div></div>
                <div class="gallery-case"><h3>"再生コントロール"</h3><div class="gallery-row">
                    <button type="button" class="icon-button" aria-label="前の曲">"⏮"</button>
                    <button type="button" class="icon-button play-button" aria-label="再生">"▶"</button>
                    <button type="button" class="icon-button" aria-label="次の曲">"⏭"</button>
                    <button type="button" class="icon-button play-button" aria-label="再生（無効）" disabled>"▶"</button>
                </div></div>
                <div class="gallery-case"><h3>"ナビゲーション項目"</h3>
                    <nav class="primary-nav gallery-nav" aria-label="ナビゲーション項目の見本">
                        <a class="nav-link" href="#" aria-current="page"><span aria-hidden="true" class="nav-icon">"♫"</span><span>"選択中"</span></a>
                        <a class="nav-link" href="#"><span aria-hidden="true" class="nav-icon">"▣"</span><span>"未選択"</span></a>
                    </nav>
                </div>
                <div class="gallery-case"><h3>"TextField"</h3>
                    <TextField id="gallery-field" label="通常" placeholder="曲・アルバム・アーティスト" />
                    <TextField id="gallery-field-disabled" label="無効" placeholder="準備中" disabled=true />
                </div>
                <div class="gallery-case"><h3>"ErrorNotice"</h3>
                    <ErrorNotice message="サーバーの起動を確認して、もう一度お試しください。".to_string() />
                </div>
                <div class="gallery-case"><h3>"Dialog"</h3>
                    <Button secondary=true on_click=Callback::new(move |_| dialog_open.set(true))>"ダイアログを開く"</Button>
                    <p class="gallery-token">"Tab/Shift+Tabの循環、Escape・閉じるでのフォーカス復帰を確認できます。"</p>
                </div>
            </div>
            <div class="gallery-case" style:margin-top="var(--space-4)"><h3>"EmptyState"</h3>
                <EmptyState title="あなたの音楽を集めよう" description="取り込み設定から音源フォルダを指定すると、ここに楽曲が並びます。">
                    <a class="button-link" href="#">"取り込み設定へ"</a>
                    <a class="text-link" href="#">"取り込み状況を見る"</a>
                </EmptyState>
            </div>
        </section>
        <Dialog open=dialog_open title="ダイアログの見本">
            <p>"背景は操作できず、フォーカスはダイアログ内を循環します。"</p>
            <p><a href="#">"ダイアログ内のリンク"</a></p>
        </Dialog>
    }
}

#[component]
fn Screens() -> impl IntoView {
    let current = RwSignal::new(SCREENS[0].0);
    view! {
        <section class="gallery-section">
            <h2>"画面プレビュー（390px / 1280px）"</h2>
            <div class="gallery-tabs" role="group" aria-label="プレビューする画面">
                {SCREENS.into_iter().map(|(path, label)| view! {
                    <button type="button" class:secondary=move || current.get() != path
                        aria-pressed=move || (current.get() == path).to_string()
                        on:click=move |_| current.set(path)>{label}</button>
                }).collect_view()}
            </div>
            <div class="gallery-frames">
                <figure><figcaption>"390px（実寸）"</figcaption>
                    <iframe class="gallery-frame" title="390px幅の画面プレビュー" width="390" height="844" src=move || current.get()></iframe>
                </figure>
                <figure><figcaption>"1280px（50%縮小）"</figcaption>
                    <div class="gallery-scaled">
                        <iframe class="gallery-frame" title="1280px幅の画面プレビュー" src=move || current.get()></iframe>
                    </div>
                </figure>
            </div>
        </section>
    }
}
