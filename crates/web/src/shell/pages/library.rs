use crate::components::{EmptyState, TextField};
use leptos::prelude::*;
use leptos_router::components::A;

/// ライブラリ画面（準備中）。検索欄は無効状態で表示し、取り込み設定への導線を出す。
#[component]
pub(crate) fn Library() -> impl IntoView {
    view! {
        <TextField id="library-search" label="ライブラリ内を検索" placeholder="曲・アルバム・アーティスト（準備中）" disabled=true />
        <EmptyState title="あなたの音楽を集めよう" description="取り込み設定から音源フォルダを指定すると、ここに楽曲が並びます。">
            <A href="/settings" attr:class="button-link">"取り込み設定へ"</A>
            <A href="/imports" attr:class="text-link">"取り込み状況を見る"</A>
        </EmptyState>
    }
}
