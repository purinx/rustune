use crate::components::ErrorNotice;
use leptos::prelude::*;
use leptos_router::components::A;
use rustune_contracts::HealthResponse;

/// 設定画面。今の段階で、実際にサーバーと通信する唯一の画面。
///
/// 「接続を確認」を押すと[`check_connection`]が`GET /api/v1/health`を呼び、
/// 応答の`service`が`"rustune"`なら接続成功と表示する。
/// - `checking`：通信中のフラグ。通信中はボタンを無効にして、二重送信を防ぐ。
/// - `status`：結果の文言。`role="status"`と`aria-live="polite"`の領域に出すので、スクリーンリーダーが読み上げる。
/// - `error`：失敗したときだけ`Some`になり、`ErrorNotice`を表示する。`None`のときは何も描画しない。
#[component]
pub(crate) fn Settings() -> impl IntoView {
    let checking = RwSignal::new(false);
    let status = RwSignal::new("接続を確認してください".to_string());
    let error = RwSignal::new(None::<String>);
    view! {
        <section class="settings-card"><h2>"取り込み設定"</h2><p class="muted">"音源フォルダと自動取り込みの設定は準備中です。"</p>
            <A href="/imports" attr:class="text-link">"取り込み状況へ"</A></section>
        <section class="settings-card"><h2>"サーバー接続"</h2><p role="status" aria-live="polite">{move || status.get()}</p>
            <button type="button" disabled=move || checking.get() on:click=move |_| check_connection(checking, status, error)>
                {move || if checking.get() { "確認中…" } else { "接続を確認" }}
            </button>
            {move || error.get().map(|message| view! { <ErrorNotice message=message /> })}
        </section>
    }
}

/// 「接続を確認」のクリック処理。通信中なら何もしない。
///
/// 状態を「接続中」にしてから[`fetch_health`]を非同期で実行し、結果に応じて`status`と`error`を更新する。
/// - `get_untracked()`は、依存を登録せずに値だけを読む（クリック処理の中なので再実行は不要）。
/// - `spawn_local`は、ブラウザのイベントループ上で`async`ブロックを実行する。
///   WASMはスレッドを使わないため、`await`の間は他の処理に制御を返す。
fn check_connection(
    checking: RwSignal<bool>,
    status: RwSignal<String>,
    error: RwSignal<Option<String>>,
) {
    if checking.get_untracked() {
        return;
    }
    checking.set(true);
    error.set(None);
    status.set("接続中…".into());
    leptos::task::spawn_local(async move {
        match fetch_health().await {
            Ok(health) if health.service == "rustune" => {
                status.set("Rustuneサーバーに接続しました".into())
            }
            _ => {
                status.set("接続できませんでした".into());
                error.set(Some(
                    "サーバーの起動を確認して、もう一度お試しください。".into(),
                ));
            }
        }
        checking.set(false);
    });
}

/// `GET /api/v1/health`を呼び、応答を[`HealthResponse`]として読む。
///
/// `?`演算子で、通信・HTTPステータス・JSON解析のどこで失敗しても1つの`Err`にまとめて返す。
/// API呼び出しを`web::api`へ集約するのはタスク13で行う（MODULE_DESIGN §14）。
async fn fetch_health() -> Result<HealthResponse, gloo_net::Error> {
    let response = gloo_net::http::Request::get("/api/v1/health")
        .send()
        .await?;
    if !response.ok() {
        return Err(gloo_net::Error::GlooError("unavailable".into()));
    }
    response.json::<HealthResponse>().await
}
