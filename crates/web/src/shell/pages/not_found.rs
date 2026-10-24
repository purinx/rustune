use crate::components::EmptyState;
use crate::shell::nav::NOT_FOUND_TITLE;
use leptos::prelude::*;
use leptos_router::components::A;

/// どのrouteにも一致しないURLで表示する画面。見出しは`title_for_path`と同じ`NOT_FOUND_TITLE`を使う。
#[component]
pub(crate) fn NotFound() -> impl IntoView {
    view! { <EmptyState title=NOT_FOUND_TITLE description="ナビゲーションから移動先を選んでください。">
        <A href="/library" attr:class="button-link">"ライブラリへ"</A>
    </EmptyState> }
}
