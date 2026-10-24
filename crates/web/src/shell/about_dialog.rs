use crate::components::Dialog;
use leptos::prelude::*;

/// 「Rustuneについて」ダイアログ。開閉は、サイドバーのボタンと共有する`open`で制御する。
/// 外部リンクは`target="_blank"`で新しいタブに開き、`rel="noopener noreferrer"`で開いた先からこのページを操作させない。
#[component]
pub(crate) fn AboutDialog(open: RwSignal<bool>) -> impl IntoView {
    view! {
        <Dialog open=open title="Rustuneについて">
            <p>"購入した音楽を自分のライブラリで管理する、開発中の音楽アプリです。"</p>
            <p><a href="https://www.apple.com/jp/itunes/" target="_blank" rel="noopener noreferrer">"iTunesの案内をブラウザで開く ↗"</a></p>
        </Dialog>
    }
}
