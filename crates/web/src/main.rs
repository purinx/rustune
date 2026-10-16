use leptos::{mount::mount_to_body, prelude::*};
use rustune_contracts::HealthResponse;

fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(App);
}

#[component]
fn App() -> impl IntoView {
    let status = RwSignal::new("接続を確認してください".to_string());
    let checking = RwSignal::new(false);

    let check_connection = move |_| {
        checking.set(true);
        status.set("接続中…".into());
        leptos::task::spawn_local(async move {
            let result = async {
                let response = gloo_net::http::Request::get("/api/v1/health")
                    .send()
                    .await?;
                if !response.ok() {
                    return Err(gloo_net::Error::GlooError("server unavailable".into()));
                }
                response.json::<HealthResponse>().await
            }
            .await;
            status.set(match result {
                Ok(health) if health.service == "rustune" => "Rustuneサーバーに接続しました".into(),
                _ => "接続できませんでした。サーバーの起動を確認してください。".into(),
            });
            checking.set(false);
        });
    };

    view! {
        <main class="foundation">
            <header class="brand"><img src="/icon.svg" alt="" width="40" height="40" />"Rustune"</header>
            <section class="welcome" aria-labelledby="welcome-title">
                <p class="eyebrow">"YOUR MUSIC. YOUR SPACE."</p>
                <h1 id="welcome-title">"好きな音楽を、あなたの場所に。"</h1>
                <p class="description">"購入した音楽を集めて、整理して、聴く。"</p>
                <div class="connection">
                    <span class="connection-label">"開発基盤の接続確認"</span>
                    <p role="status" aria-live="polite">{move || status.get()}</p>
                    <button on:click=check_connection disabled=move || checking.get()>
                        {move || if checking.get() { "確認中…" } else { "接続を確認" }}
                    </button>
                </div>
                <p class="stage-note">"ライブラリの取り込み・再生機能は、これから実装します。"</p>
            </section>
        </main>
    }
}
