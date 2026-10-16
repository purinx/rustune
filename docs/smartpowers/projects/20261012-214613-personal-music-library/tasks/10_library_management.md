# 10: ライブラリ操作・削除・除外

Status: Planned
Project: 20261012-214613-personal-music-library
Requirements: REQ-005, REQ-006, REQ-009, REQ-010, REQ-011, NFR-003, NFR-005
Depends on: 09
Parallel group: None

## Objective

検索・整理・削除を永続化し、取り込みとの競合でも意図を保持する。

## Design references

MODULE_DESIGN §3・11。プロジェクトの承認済みPRD・DESIGN_DOC・MODULE_DESIGNを前提とする。

## Scope

### Included

以下の実装手順と検証を含む。

### Excluded

HTTP配信leaseの具象実装、画面操作。

## Expected code areas

crates/server/src/library/、関連db操作・統合テスト

## Implementation steps

1. ready曲の検索・ページング、アルバム/アーティスト集計、有効metadataとグループ更新を実装する。
2. 編集PATCHとrevision、プレイリストentryと順序、お気に入りを実装する。
3. 除外登録、deleting/deleted、参照整理、削除再試行、除外解除を実装する。
4. 09のmutation mutexに接続し、media leaseの通知境界を11へ引き継ぐ。

## Verification

- 日本語・LIKE文字・編集後再走査・グループ移動をテストする。
- プレイリスト重複曲・競合・再起動、取り込み中削除・解除をテストする。

検証コマンドの未指定部分は実装したテスト名に合わせて具体化し、実行結果・環境をこのファイルへ追記する。上記は予定であり、現在の合格結果ではない。

## Acceptance criteria

手動編集を上書きせず、削除曲を勝手に再取り込みせず、リスト削除で音源を消さない。

## Handoff notes

- 前提タスクの成果と検証記録を確認してから着手する。未解決の設計変更は該当承認ゲートへ戻す。
- 予定領域外の変更、共有Cargo設定・router・migrationへの変更が必要なら統合担当へ集約し、並行タスクの所有範囲を侵さない。
- UI変更時はdesign.mdを全文読み、依存関係・連携を変更したらMermaid図も更新する。
- 完了時にStatusをDoneへ更新し、変更ファイル、検証結果、未解決事項を記録する。購入音源・購入者情報はコミットしない。
