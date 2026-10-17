# Rustune — Rust・WASMで作る個人用音楽ライブラリ

Project: 20261012-214613-personal-music-library
Status: Active
Created: 2026-10-12T21:46:13+09:00
Slug: personal-music-library

Last updated: 2026-10-18
Phase: Implementation

## 方針

iTunes Storeで購入・ダウンロードした、ブラウザで再生可能な`.m4a`（AAC）ファイルを、ミュージックアプリのメディアフォルダからスキャンして個人ライブラリへコピーし、管理・再生する。iTunes Search APIはメタデータとジャケットの補完にのみ使う。個人利用のみ。バックエンドはRust、フロントエンドはRust/WASMを中心とする。

## 現在の成果物と次のゲート

- PRD.md: Approved（2026-10-12 by user）。
- 承認済み要件変更：REQ-013はAPIが返す低解像度ジャケットを許容。PRDと設計へ反映済み。
- DESIGN_DOC.md: Approved（2026-10-16 by user）。Leptos CSR + Axum/Tokio + SQLite/SQLxを中心とする全体設計。
- MODULE_DESIGN.md: Approved（2026-10-16 by user）。モジュール境界、DTO、DB制約、取り込み・削除・復旧、WASM再生、CIの詳細設計。両設計のLGTMを受領。
- 設計修正：ユーザーの賛同に基づきcoreをdomainへ改名。ドメイン型と不変条件はdomain、通信DTOはcontractsが所有する。
- 実装タスク16件の粒度・依存順はユーザー合意済み（2026-10-16）。
- tasks/PLAN.md: Approved（2026-10-16 by user）。実装開始指示を受領。
- タスク01: Done。Rust 1.99.1・4クレートworkspace・CI・WASM最小画面・HTTP接続を実装。native/WASM Clippy、Rustテスト、Trunk release、独立Chromeの390px/1280px確認に合格。
- 承認済み変更（2026-10-17 by user）：DRM専用判定と再取得音源の同一性保証・比較ゲートを削除。PRD・全体設計・詳細設計・タスク02/06/16・PLANへ同期した。同じファイルの別パスコピー・再スキャンの重複防止と専用保存先へのコピーは維持。
- タスク02: Done。購入由来タグ規則・採用パーサー・上限の製品適用方針を記録。合成12件と指定実音源の別パスコピー・タグ差分1件、fmt/Clippyに成功。製品inspectionの実装は06へ引き継ぐ。
- 承認済み変更（2026-10-18 by user）：インストール用の最小Web App Manifestを導入する。Service Worker・オフライン再生は対象外のまま。PRD・設計・タスク03/08・PLANへ反映。
- タスク03: In Progress。合成AACと指定実音源のRange配信、Chrome headless再生・シーク・Media Session登録の自動確認は合格。ユーザーが検証ページを開けたことを確認。最小Manifestへ変更し、インストール・Dock・実メディアキー等の手動確認を続ける。
- タスク04は03の検証ゲート完了待ち。04完了後に05/06/07/08を並列着手できる。
- 承認済みPRDと設計書に従い、依存順で実装する。
