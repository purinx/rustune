use super::pages::{Collection, Exclusions, Imports, Library, NotFound, Settings};
use leptos::prelude::*;
use leptos_router::{components::*, path};

/// URLと画面の対応。URLに一致する画面を1つだけ描画し、どれにも一致しなければ[`NotFound`]を描画する。
///
/// 見出しを決める`nav::title_for_path`は、ここと同じ対応を前提にしている。routeを増やしたら両方を直す。
/// - `view=Library`のようにコンポーネント名を渡す書き方と、`view=|| view! { ... }`のように
///   引数なしのクロージャを渡す書き方がある。後者は、propsを渡して描画したいときに使う。
/// - `path!("/albums/:key")`の`:key`は、任意の1階層に一致する。値はまだ読み取らない。
#[component]
pub(crate) fn AppRoutes() -> impl IntoView {
    view! {
        <Routes fallback=NotFound>
            <Route path=path!("") view=|| view! { <Redirect path="/library" /> } />
            <Route path=path!("/library") view=Library />
            <Route path=path!("/albums") view=|| view! { <Collection kind="アルバム" /> } />
            <Route path=path!("/albums/:key") view=|| view! { <Collection kind="アルバム詳細" /> } />
            <Route path=path!("/artists") view=|| view! { <Collection kind="アーティスト" /> } />
            <Route path=path!("/artists/:key") view=|| view! { <Collection kind="アーティスト詳細" /> } />
            <Route path=path!("/playlists") view=|| view! { <Collection kind="プレイリスト" /> } />
            <Route path=path!("/playlists/:id") view=|| view! { <Collection kind="プレイリスト詳細" /> } />
            <Route path=path!("/favorites") view=|| view! { <Collection kind="お気に入り" /> } />
            <Route path=path!("/imports") view=Imports />
            <Route path=path!("/settings") view=Settings />
            <Route path=path!("/exclusions") view=Exclusions />
        </Routes>
    }
}
