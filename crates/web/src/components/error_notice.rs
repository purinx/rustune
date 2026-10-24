use leptos::prelude::*;

/// エラーメッセージ。`role="alert"`により、表示された時点でスクリーンリーダーが読み上げる。
/// `message`は`String`（所有する文字列）で受け取る。実行時に作った文言を渡せるようにするため。
#[component]
pub fn ErrorNotice(message: String) -> impl IntoView {
    view! { <p class="error-notice" role="alert">{message}</p> }
}
