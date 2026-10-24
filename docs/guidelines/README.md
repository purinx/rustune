# Rustune コーディングガイドライン

既存コード（`main`の`crates/domain`・`crates/contracts`・`crates/server`・`crates/web`と、タスク05〜08ブランチの実装）から読み取った書き方を、今後の実装と修正で守る規約として明文化したもの。

| 文書 | 内容 |
| --- | --- |
| [coding-rules.md](coding-rules.md) | Rust全般：命名、型設計、エラー、コメント、import、定数、unsafe・panic |
| [responsibility-separation.md](responsibility-separation.md) | 責務分離：レイヤー、モジュール・ファイル分割、関数分割の基準 |
| [testing.md](testing.md) | テストの配置・命名・fixture・検証コマンド |
| [web-ui.md](web-ui.md) | Leptos・CSSの実装規約 |

## 優先順位

1. 承認済みの`PRD.md`・`DESIGN_DOC.md`・`MODULE_DESIGN.md`（`docs/smartpowers/projects/20261012-214613-personal-music-library/`）
2. UIの視覚仕様は`design.md`
3. `AGENTS.md`
4. 本ガイドライン

本ガイドラインは設計書のモジュール境界を変えない。設計書が定めたモジュール（`server::storage`など）の**内側**をどう分けるかを定める。設計書と衝突する場合は設計書を優先し、本ガイドラインを直す。

## 機械的に強制される規約

| 項目 | 設定箇所 | 検査 |
| --- | --- | --- |
| フォーマット | rustfmt既定値（`rustfmt.toml`なし） | `cargo fmt --all -- --check` |
| Lint | Clippy既定、警告はエラー | `cargo clippy ... -- -D warnings`（native・WASM別） |
| unsafe禁止 | `Cargo.toml`の`[workspace.lints.rust] unsafe_code = "forbid"` | コンパイル |
| ツールチェーン | `rust-toolchain.toml`（Rust 1.99.1、edition 2024） | `rustup show` |

各crateの`Cargo.toml`には必ず`[lints] workspace = true`を書き、workspaceのlint設定を継承する。検査コマンドの一覧は[testing.md](testing.md#検証コマンド)を参照する。
