use leptos::prelude::*;

/// 表示するデータがないときの案内。見出し・説明と、次の操作（`children`のリンクやボタン）を並べる。
/// `aria-hidden="true"`の音符は装飾なので、スクリーンリーダーには読ませない。
#[component]
pub fn EmptyState(
    title: &'static str,
    description: &'static str,
    children: Children,
) -> impl IntoView {
    view! { <section class="empty-state"><div class="empty-art" aria-hidden="true">"♫"</div>
    <h2>{title}</h2><p>{description}</p><div class="actions">{children()}</div></section> }
}
