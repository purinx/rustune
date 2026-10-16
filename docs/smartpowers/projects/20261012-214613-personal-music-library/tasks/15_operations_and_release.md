# 15: 自動起動・配布・復元・ログ運用

Status: Planned
Project: 20261012-214613-personal-music-library
Requirements: REQ-017, NFR-005, NFR-008, NFR-010
Depends on: 13
Parallel group: C

## Objective

Macログイン後に利用でき、更新失敗やデータ消失から復帰できる運用を用意する。

## Design references

MODULE_DESIGN §4・16、DESIGN_DOC Operations/Deployment。プロジェクトの承認済みPRD・DESIGN_DOC・MODULE_DESIGNを前提とする。

## Scope

### Included

以下の実装手順と検証を含む。

### Excluded

本番データの削除、無断のOS起動設定変更、NAS/VPS運用。

## Expected code areas

配布スクリプト、LaunchAgentテンプレート、運用文書、serverのログ・終了処理

## Implementation steps

1. バイナリ・WASM資産・migrationを同じリリースへまとめる。
2. ユーザーLaunchAgentの設定・解除・異常再起動・絶対パス指定を手順化する。
3. JSONログのredaction・10 MiB×5本ローテーション・ジョブ履歴90日整理を実装する。
4. 停止バックアップ、同版復元、更新・migration失敗時の復旧手順を記載する。

## Verification

- 実Macで設定→ログイン→health→Dock起動、異常終了再起動、解除を確認する。
- バックアップを別の一時保存先へ復元し、曲・編集・リスト・除外を確認する。
- 通常ログ・公開エラーに購入者値がないことを検証する。

検証コマンドの未指定部分は実装したテスト名に合わせて具体化し、実行結果・環境をこのファイルへ追記する。上記は予定であり、現在の合格結果ではない。

## Acceptance criteria

起動・解除・更新・復元手順を再現でき、元Musicフォルダを変更しない。

## Handoff notes

- 前提タスクの成果と検証記録を確認してから着手する。未解決の設計変更は該当承認ゲートへ戻す。
- 予定領域外の変更、共有Cargo設定・router・migrationへの変更が必要なら統合担当へ集約し、並行タスクの所有範囲を侵さない。
- UI変更時はdesign.mdを全文読み、依存関係・連携を変更したらMermaid図も更新する。
- 完了時にStatusをDoneへ更新し、変更ファイル、検証結果、未解決事項を記録する。購入音源・購入者情報はコミットしない。
