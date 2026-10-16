# 16: 全体受け入れと初期版の完成判定

Status: Planned
Project: 20261012-214613-personal-music-library
Requirements: REQ-001, REQ-002, REQ-003, REQ-004, REQ-005, REQ-006, REQ-007, REQ-008, REQ-009, REQ-010, REQ-011, REQ-012, REQ-013, REQ-014, REQ-015, REQ-016, REQ-017, NFR-001, NFR-002, NFR-003, NFR-004, NFR-005, NFR-006, NFR-007, NFR-008, NFR-009, NFR-010
Depends on: 14, 15
Parallel group: None

## Objective

全要件を実証し、未検証や延期を隠さず初期版の完成を判断する。

## Design references

PRD Success measures、DESIGN_DOC Testing strategy、MODULE_DESIGN §17、design.md。プロジェクトの承認済みPRD・DESIGN_DOC・MODULE_DESIGNを前提とする。

## Scope

### Included

以下の実装手順と検証を含む。

### Excluded

初期版外の機能追加、未検証項目を仮合格にすること。

## Expected code areas

全体E2E・障害注入・性能検証、受け入れ記録、プロジェクト状態

## Implementation steps

1. 購入10曲以上で取り込み・再起動・整理・再生・削除除外を通し確認する。
2. コピー・commit・削除の障害、容量・権限・DRM・途中ファイルを検証する。
3. 1万曲・取り込み有無の検索p95とAAC再生開始p95を固定条件で測る。
4. 3実ブラウザ・2画面幅・キーボード・Chromeインストール・OS操作・ログイン起動を確認する。
5. 全Must/NFRとShouldの結果・延期承認を記録する。必要な修正後に関連チェックを再実施し、全体合格後にPROJECTをCompletedへ更新する。

## Verification

- native/WASM別Clippy、rustfmt、nativeテスト、Trunk release build、ブラウザテスト。
- 検索100回でp95≤1秒、再生開始30回でp95≤2秒。OS・機材・音源条件を記録する。
- バックアップ復元、個人情報非露出、design.md準拠を確認する。

検証コマンドの未指定部分は実装したテスト名に合わせて具体化し、実行結果・環境をこのファイルへ追記する。上記は予定であり、現在の合格結果ではない。

## Acceptance criteria

全MustとNFRが合格し、Shouldの完了または承認済み延期が明確。残余リスク・再現手順・配布物が揃う。

## Handoff notes

- 前提タスクの成果と検証記録を確認してから着手する。未解決の設計変更は該当承認ゲートへ戻す。
- 予定領域外の変更、共有Cargo設定・router・migrationへの変更が必要なら統合担当へ集約し、並行タスクの所有範囲を侵さない。
- UI変更時はdesign.mdを全文読み、依存関係・連携を変更したらMermaid図も更新する。
- 完了時にStatusをDoneへ更新し、変更ファイル、検証結果、未解決事項を記録する。購入音源・購入者情報はコミットしない。
