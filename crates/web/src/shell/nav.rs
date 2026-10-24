//! ナビゲーションの定義と、URLから見出しを決める純粋関数。
//!
//! DOMやsignalを使わないので、ブラウザなしの`cargo test`で検証できる。
//!
//! `const NAV: [(&str, &str, &str); 8]`は、(パス, 表示名, アイコン)の組を8個持つ固定長の配列。
//! `&str`は文字列への参照で、文字列リテラルはプログラム全体の寿命（`'static`）を持つ。

/// サイドバー（狭い幅では下部ナビゲーション）に並べる項目。(パス, 表示名, アイコン)。
pub(crate) const NAV: [(&str, &str, &str); 8] = [
    ("/library", "ライブラリ", "♫"),
    ("/albums", "アルバム", "▣"),
    ("/artists", "アーティスト", "♙"),
    ("/playlists", "プレイリスト", "≡"),
    ("/favorites", "お気に入り", "♡"),
    ("/imports", "取り込み", "↓"),
    ("/settings", "設定", "⚙"),
    ("/exclusions", "除外した音源", "⊘"),
];

/// 詳細画面（`/:key`・`/:id`）を持つセクション。`Routes`の定義と一致させる。
const DETAIL_SECTIONS: [&str; 3] = ["/albums", "/artists", "/playlists"];

/// `Routes`が実際に描画する画面に対応した見出しを返す。
/// 既知セクション配下でも未定義のパス（`/library/missing`など）は`NotFound`と同じ見出しにする。
/// ルーターは末尾スラッシュ1つを無視して照合するため（`/library/`はライブラリを描画）、同様に扱う。
pub(crate) fn title_for_path(path: &str) -> &'static str {
    // 末尾の`/`を1つだけ取り除く。`strip_suffix`は、一致しなければ`None`を返すので、そのときは元の値を使う。
    let path = path.strip_suffix('/').unwrap_or(path);
    // NAVを先頭から調べ、条件に合う最初の項目の表示名を返す。見つからなければNOT_FOUND_TITLE。
    // 条件は「一覧のパスと完全一致」か、「詳細を持つセクションの直下に、空でない1階層だけが続く」こと。
    // `strip_prefix`は接頭辞を除いた残りを`Option`で返し、`and_then`は`Some`のときだけ次の処理へ進む。
    NAV.iter()
        .find(|(href, _, _)| {
            path == *href
                || (DETAIL_SECTIONS.contains(href)
                    && path
                        .strip_prefix(href)
                        .and_then(|rest| rest.strip_prefix('/'))
                        .is_some_and(|key| !key.is_empty() && !key.contains('/')))
        })
        .map(|(_, label, _)| *label)
        .unwrap_or(NOT_FOUND_TITLE)
}

pub(crate) const NOT_FOUND_TITLE: &str = "ページが見つかりません";

#[cfg(test)]
mod tests {
    use super::{NOT_FOUND_TITLE, title_for_path};

    #[test]
    fn known_collection_and_detail_paths_use_section_titles() {
        assert_eq!(title_for_path("/library"), "ライブラリ");
        assert_eq!(title_for_path("/library/"), "ライブラリ");
        assert_eq!(title_for_path("/albums/"), "アルバム");
        assert_eq!(title_for_path("/albums/key/"), "アルバム");
        assert_eq!(title_for_path("/albums"), "アルバム");
        assert_eq!(title_for_path("/albums/key"), "アルバム");
        assert_eq!(title_for_path("/artists/key"), "アーティスト");
        assert_eq!(title_for_path("/playlists/0f8e"), "プレイリスト");
        assert_eq!(title_for_path("/exclusions"), "除外した音源");
    }

    #[test]
    fn unsupported_paths_under_known_sections_are_not_found() {
        for path in [
            "/library/missing",
            "/albums/key/extra",
            "/albums//",
            "/favorites/x",
            "/settings/advanced",
            "/librar",
            "/libraryx",
            "/unknown",
        ] {
            assert_eq!(title_for_path(path), NOT_FOUND_TITLE, "{path}");
        }
    }
}
