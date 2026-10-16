# 11: API・音声配信・サーバー統合

Status: Planned
Project: 20261012-214613-personal-music-library
Requirements: REQ-001, REQ-002, REQ-005, REQ-006, REQ-007, REQ-009, REQ-010, REQ-011, REQ-014, NFR-004, NFR-008, NFR-010
Depends on: 10
Parallel group: None

## Objective

サービスを同一originのAPIで公開し、保存音源をシーク可能に配信する。

## Design references

MODULE_DESIGN §12、DESIGN_DOC Interfaces。プロジェクトの承認済みPRD・DESIGN_DOC・MODULE_DESIGNを前提とする。

## Scope

### Included

以下の実装手順と検証を含む。

### Excluded

フロントの状態管理、iTunes補完APIの製品実装。

## Expected code areas

crates/server/src/http/、media/、起動処理、静的配信・HTTP結合テスト

## Implementation steps

1. DTO変換、入力制約、revision競合、session/state/healthを含むルートを実装する。
2. loopback、Host/Origin、CSRF、公開エラー、request ID、リクエスト上限を接続する。
3. ID解決・ready検査・Range/HEAD・media leaseと削除cancelを実装する。
4. 静的資産配信・SPA fallbackとAPI404を分離し、bootstrap/recovery/workerを統合する。

## Verification

- 200/206/416、suffix/open range、HEAD、multi-range、末尾moovを確認する。
- Originなしaudio要求とcross-site変更拒否、任意パス拒否、購入者値の非露出を確認する。
- 配信開始と削除の競合、lease drop、server再起動をテストする。

検証コマンドの未指定部分は実装したテスト名に合わせて具体化し、実行結果・環境をこのファイルへ追記する。上記は予定であり、現在の合格結果ではない。

## Acceptance criteria

実UI接続可能なAPI一式が動き、Range再生と削除停止が矛盾しない。

## Handoff notes

- 前提タスクの成果と検証記録を確認してから着手する。未解決の設計変更は該当承認ゲートへ戻す。
- 予定領域外の変更、共有Cargo設定・router・migrationへの変更が必要なら統合担当へ集約し、並行タスクの所有範囲を侵さない。
- UI変更時はdesign.mdを全文読み、依存関係・連携を変更したらMermaid図も更新する。
- 完了時にStatusをDoneへ更新し、変更ファイル、検証結果、未解決事項を記録する。購入音源・購入者情報はコミットしない。
