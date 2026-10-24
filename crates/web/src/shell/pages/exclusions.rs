use crate::components::EmptyState;
use leptos::prelude::*;
use leptos_router::components::A;

/// 除外した音源の画面（準備中）。
#[component]
pub(crate) fn Exclusions() -> impl IntoView {
    view! { <EmptyState title="除外した音源はありません" description="取り込みから除外した音源の管理機能は準備中です。">
        <A href="/library" attr:class="button-link secondary">"ライブラリへ"</A>
    </EmptyState> }
}
