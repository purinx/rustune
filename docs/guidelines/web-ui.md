# Web UI 実装規約

視覚仕様は`design.md`、画面の要件は`PRD.md`に従う（`AGENTS.md`の「UI実装」）。本書はLeptosとCSSのコードの書き方だけを定める。

## Leptos

- componentは`#[component]`付きの`UpperCamelCase`関数で、`impl IntoView`を返す。
- 状態は`RwSignal`で持ち、表示は`move || signal.get()`で参照する。
- 非同期処理は`leptos::task::spawn_local`で起動する。通信の中身は`web::api`の関数に置き、componentはその結果を状態へ反映するだけにする（[responsibility-separation.md](responsibility-separation.md#web)）。
- 共通部品（`Button`、`Dialog`、`EmptyState`、`ErrorNotice`、`TextField`）は`web::components`に置き、画面ごとに同じ見た目の部品を作り直さない。
- サーバーの`ApiError.code`が未知の値でも、汎用の日本語メッセージで表示する（`crates/contracts/README.md`）。
- panic時に情報を得られるよう、`main`で`console_error_panic_hook::set_once()`を呼ぶ。

## 文言・アクセシビリティ

- 画面の文言は日本語で書く。`view!`内の文字列リテラルは`"…"`で囲む。
- 状態の変化を伝える要素には`role="status"`と`aria-live="polite"`を付ける。
- 装飾目的の画像・アイコンは`alt=""`または`aria-hidden="true"`にする。
- ページ遷移時は見出しへフォーカスを移し、`document.title`を「<画面名> — Rustune」にする（`shell`の実装）。
- キーボード操作とフォーカス表示（`:focus-visible`）を全ての操作要素で確認する。

## CSS

- 色・文字サイズ・角丸・余白・影は`:root`のCSS変数（`--background`、`--accent`、`--radius-card`、`--space`など）で定義し、各ルールでは変数を参照する。値を直接書かない。
  - `main`の`style.css`には`#000000`・`font-size: 14px`などの直接指定が残っている。変更時にトークンへ移す。
- 余白は`--space`（8px）の倍数で書く（`calc(var(--space) * 4)`）。
- クラス名は`kebab-case`で、画面上の役割を表す名前にする（`.connection-label`、`.stage-note`）。
- レスポンシブ対応は幅390pxと1280pxで主要操作を確認する。ブレークポイントは`design.md`に従う。
