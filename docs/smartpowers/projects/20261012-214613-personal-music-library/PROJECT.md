# Rustune — Rust・WASMで作る個人用音楽ライブラリ

Project: 20261012-214613-personal-music-library
Status: Active
Created: 2026-10-12T21:46:13+09:00
Slug: personal-music-library

Last updated: 2026-10-16
Phase: Planning

## 方針

iTunes Storeで購入・ダウンロードしたDRMなしの`.m4a`ファイルを、ミュージックアプリのメディアフォルダからスキャンして個人ライブラリへコピーし、管理・再生する。DRM付き（`.m4p`、Apple Musicのサブスク曲）は対象外。iTunes Search APIはメタデータとジャケットの補完にのみ使う。個人利用のみ。バックエンドはRust、フロントエンドはRust/WASMを中心とする。

## 現在の成果物と次のゲート

- PRD.md: Approved（2026-10-12 by user）。
- 承認済み要件変更：REQ-013はAPIが返す低解像度ジャケットを許容。PRDと設計へ反映済み。
- DESIGN_DOC.md: Approved（2026-10-16 by user）。Leptos CSR + Axum/Tokio + SQLite/SQLxを中心とする全体設計。
- MODULE_DESIGN.md: Approved（2026-10-16 by user）。モジュール境界、DTO、DB制約、取り込み・削除・復旧、WASM再生、CIの詳細設計。両設計のLGTMを受領。
- 設計修正：ユーザーの賛同に基づきcoreをdomainへ改名。ドメイン型と不変条件はdomain、通信DTOはcontractsが所有する。
- 次のゲート：実装タスクの粒度と依存順の合意。購入曲識別・Chrome挙動等の実機検証事項は承認済み設計に記載したとおり維持する。
- 実装計画・実装は未着手。承認済みPRDは変更していない。
