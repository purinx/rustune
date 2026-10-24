use leptos::prelude::*;

/// ラベル付きの検索入力欄。`label`の`for`と`input`の`id`を一致させ、ラベルのクリックで入力欄へ移れるようにする。
/// 入力値の保持や検索処理はまだ持たない（ライブラリ画面の検索はタスク13で接続する）。
#[component]
pub fn TextField(
    id: &'static str,
    label: &'static str,
    placeholder: &'static str,
    #[prop(default = false)] disabled: bool,
) -> impl IntoView {
    view! { <div class="field"><label for=id>{label}</label>
    <input id=id type="search" placeholder=placeholder disabled=disabled /></div> }
}
