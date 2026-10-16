# 13: 各画面のAPI接続と同期

Status: Planned
Project: 20261012-214613-personal-music-library
Requirements: REQ-001, REQ-002, REQ-005, REQ-006, REQ-009, REQ-010, REQ-011, REQ-014, REQ-015, NFR-002, NFR-009
Depends on: 12
Parallel group: None

## Objective

取り込みから検索・整理・再生までUIだけで完了できるようにする。

## Design references

design.md全文、MODULE_DESIGN §14。プロジェクトの承認済みPRD・DESIGN_DOC・MODULE_DESIGNを前提とする。

## Scope

### Included

以下の実装手順と検証を含む。

### Excluded

API補完、OS自動起動設定の変更。

## Expected code areas

crates/web/src/api/、sync/、pages/、画面E2E

## Implementation steps

1. 型付きApiClientと接続状態、timeout時の結果不明・再確認を実装する。
2. ライブラリ・詳細・リスト・お気に入り・取り込み履歴・設定・除外画面を接続する。
3. 編集formのdirty/revision、250ms検索debounce、古い応答排除を実装する。
4. 2秒/15秒同期、visibility復帰、切断backoff、削除曲キュー整理を実装する。
5. デザイン・日本語・フォーカス・並べ替えの上下操作を実画面で確認する。

## Verification

- 購入音源追加→自動取り込み→編集→検索→プレイリスト→連続再生のE2E。
- 通信逆転・切断復帰・409・編集途中同期・削除除外解除を確認する。
- 390px/1280pxと実Chrome/Firefox/Safariの主要操作を確認する。

検証コマンドの未指定部分は実装したテスト名に合わせて具体化し、実行結果・環境をこのファイルへ追記する。上記は予定であり、現在の合格結果ではない。

## Acceptance criteria

管理操作がUIで完結し、再接続や同期で入力中の値を黙って消さない。

## Handoff notes

- 前提タスクの成果と検証記録を確認してから着手する。未解決の設計変更は該当承認ゲートへ戻す。
- 予定領域外の変更、共有Cargo設定・router・migrationへの変更が必要なら統合担当へ集約し、並行タスクの所有範囲を侵さない。
- UI変更時はdesign.mdを全文読み、依存関係・連携を変更したらMermaid図も更新する。
- 完了時にStatusをDoneへ更新し、変更ファイル、検証結果、未解決事項を記録する。購入音源・購入者情報はコミットしない。
