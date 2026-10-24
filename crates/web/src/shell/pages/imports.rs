use crate::components::{Button, EmptyState};
use leptos::prelude::*;
use leptos_router::components::A;

/// 取り込み画面（準備中）。スキャン開始ボタンは無効状態で表示する。
#[component]
pub(crate) fn Imports() -> impl IntoView {
    view! { <EmptyState title="取り込み履歴はまだありません" description="フォルダの走査と取り込み処理は準備中です。">
        <Button disabled=true>"スキャンを開始（準備中）"</Button>
        <A href="/settings" attr:class="text-link">"取り込み設定へ"</A>
    </EmptyState> }
}
