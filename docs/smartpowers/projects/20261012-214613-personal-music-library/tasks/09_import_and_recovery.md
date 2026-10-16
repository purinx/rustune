# 09: 取り込みジョブ・自動走査・復旧

Status: Planned
Project: 20261012-214613-personal-music-library
Requirements: REQ-001, REQ-002, REQ-003, REQ-004, REQ-012, REQ-014, NFR-003, NFR-005, NFR-006, NFR-007
Depends on: 05, 06, 07
Parallel group: None

## Objective

手動・自動の追加を同じ経路で処理し、中断しても整合した状態へ復旧する。

## Design references

MODULE_DESIGN §9・10。プロジェクトの承認済みPRD・DESIGN_DOC・MODULE_DESIGNを前提とする。

## Scope

### Included

以下の実装手順と検証を含む。

### Excluded

UI、HTTP handler、購入判定の未検証な緩和。

## Expected code areas

crates/server/src/import/、recovery/、関連db操作、起動接続・統合テスト

## Implementation steps

1. 永続ジョブ予約、通知とDB再確認、単一worker、60秒scheduler、10秒安定待ちを実装する。
2. 重複・除外・未変更sourceの早期skip、コピー前後検証、予約manifestとready公開を接続する。
3. 項目集計・スキップ理由・容量不足・設定変更・キャンセルを実装する。
4. 起動時のmanifest照合、孤立ファイル回収、missing音源・deleting復旧を実装する。
5. publishと削除に共有する短時間mutation mutexの境界を定義する。

## Verification

- 予約commit・rename・ready commit前後で停止し、再起動を2回行う。
- 自動有効/無効、停止中の追加、途中ファイル、同時手動要求、集計一致を検証する。
- 通常条件の追加で90秒以内に取り込み開始、走査中のHTTP処理を確認する。

検証コマンドの未指定部分は実装したテスト名に合わせて具体化し、実行結果・環境をこのファイルへ追記する。上記は予定であり、現在の合格結果ではない。

## Acceptance criteria

再走査で曲が増えず、中断ジョブが処理中のまま残らず、一時ファイルが再生対象にならない。

## Handoff notes

- 前提タスクの成果と検証記録を確認してから着手する。未解決の設計変更は該当承認ゲートへ戻す。
- 予定領域外の変更、共有Cargo設定・router・migrationへの変更が必要なら統合担当へ集約し、並行タスクの所有範囲を侵さない。
- UI変更時はdesign.mdを全文読み、依存関係・連携を変更したらMermaid図も更新する。
- 完了時にStatusをDoneへ更新し、変更ファイル、検証結果、未解決事項を記録する。購入音源・購入者情報はコミットしない。
