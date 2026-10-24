use crate::components::EmptyState;
use leptos::prelude::*;
use leptos_router::components::A;

/// アルバム・アーティスト・プレイリスト・お気に入りと、その詳細画面に共通の準備中表示。
/// `kind`で画面名を差し替える。詳細画面の`:key`・`:id`は、まだ読み取らない。
#[component]
pub(crate) fn Collection(kind: &'static str) -> impl IntoView {
    view! { <EmptyState title="音楽がまだありません" description="ライブラリへの取り込み機能は準備中です。">
        <p class="muted">{format!("{kind}の表示は、楽曲を取り込んだあとに利用できます。")}</p>
        <A href="/library" attr:class="button-link secondary">"ライブラリへ"</A>
    </EmptyState> }
}
