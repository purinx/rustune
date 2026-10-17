# 08: デザイン準拠のUI基盤

Status: Planned
Project: 20261012-214613-personal-music-library
Requirements: REQ-015, NFR-002, NFR-009
Depends on: 03
Parallel group: B

## Objective

Spotify風ガイドを守ったRustuneの画面基盤を構築する。

## Design references

design.md全文、AGENTS.md、MODULE_DESIGN §14・15。プロジェクトの承認済みPRD・DESIGN_DOC・MODULE_DESIGNを前提とする。

## Scope

### Included

以下の実装手順と検証を含む。

### Excluded

実API接続、音声再生、個別ページの業務form。

## Expected code areas

crates/web/src/shell/、components/、共通CSS・静的アイコン・index.html

## Implementation steps

1. design.mdを読み、配色・タイポグラフィ・角丸・余白・影をCSSトークンにまとめる。
2. Rustuneロゴ、sidebar、狭幅navigation、固定再生バー領域、routerを作る。
3. ボタン・入力・dialog・空状態・エラー表示とフォーカスの共通部品を作る。
4. 03の成果を使い、title・最小Web App Manifest・192px/512pxアイコン・外部リンクを設定し、Trunk成果物に含める。Service Workerは追加しない。PlayerControllerは12が所有する。

## Verification

- Trunk release buildとブラウザ表示を確認する。
- 390px/1280px、キーボードnavigation、dialogフォーカス、design.mdとの差異を確認する。

検証コマンドの未指定部分は実装したテスト名に合わせて具体化し、実行結果・環境をこのファイルへ追記する。上記は予定であり、現在の合格結果ではない。

## Acceptance criteria

主要ナビゲーションがブラウザの戻るボタンなしで使え、狭幅でも再生操作領域が隠れない。

## Handoff notes

- 2026-10-18の並列化指示に基づき、実APIや業務型を使用しないUI基盤は03の完了後に先行する。04への依存を外し、API接続・業務form・正式プレイヤーを範囲に加えない。

- 前提タスクの成果と検証記録を確認してから着手する。未解決の設計変更は該当承認ゲートへ戻す。
- 予定領域外の変更、共有Cargo設定・router・migrationへの変更が必要なら統合担当へ集約し、並行タスクの所有範囲を侵さない。
- UI変更時はdesign.mdを全文読み、依存関係・連携を変更したらMermaid図も更新する。
- 完了時にStatusをDoneへ更新し、変更ファイル、検証結果、未解決事項を記録する。購入音源・購入者情報はコミットしない。
