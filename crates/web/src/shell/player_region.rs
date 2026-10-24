use leptos::prelude::*;

/// 画面下部に固定する再生領域（準備中）。画面遷移しても消えないよう、`Routes`の外に置く。
/// ボタンは無効状態で、`aria-label`に「準備中」を含める。実際の再生処理はタスク12で接続する。
#[component]
pub(crate) fn PlayerRegion() -> impl IntoView {
    view! { <section class="player-region" aria-label="再生コントロール（準備中）">
        <div class="track-placeholder"><div class="cover-placeholder" aria-hidden="true">"♫"</div>
            <div><p>"再生する曲を選んでください"</p><span>"プレイヤーは準備中です"</span></div></div>
        <div class="player-controls">
            <button type="button" class="icon-button" aria-label="前の曲（準備中）" disabled>"⏮"</button>
            <button type="button" class="icon-button play-button" aria-label="再生（準備中）" disabled>"▶"</button>
            <button type="button" class="icon-button" aria-label="次の曲（準備中）" disabled>"⏭"</button>
        </div><span class="player-time" aria-label="再生時間未設定">"—:— / —:—"</span>
    </section> }
}
