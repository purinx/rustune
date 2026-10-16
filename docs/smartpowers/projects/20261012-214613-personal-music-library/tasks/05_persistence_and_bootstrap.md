# 05: SQLite・設定・起動基盤

Status: Planned
Project: 20261012-214613-personal-music-library
Requirements: REQ-005, REQ-009, REQ-010, REQ-011, NFR-005, NFR-008
Depends on: 04
Parallel group: B

## Objective

管理情報・ジョブ・除外を整合して保存できる土台を作る。

## Design references

MODULE_DESIGN §4・5。プロジェクトの承認済みPRD・DESIGN_DOC・MODULE_DESIGNを前提とする。

## Scope

### Included

以下の実装手順と検証を含む。

### Excluded

取り込みワーカー、Libraryサービスの業務処理、UI。

## Expected code areas

crates/server/src/db/、config.rs、bootstrap.rs、migrations/、関連統合テスト

## Implementation steps

1. 全体スキーマとUNIQUE/FK/CHECK・索引をmigrationへ記載し、migration所有をこのタスクへ集約する。
2. WAL、pool、busy timeout、ユースケース単位のRepository、revisionのCASを実装する。
3. TOMLとDB設定の所有を分け、保存先・rootの重なり、設定値の範囲を検証する。
4. プロセスロック、起動・終了の共通基盤を用意する。recoveryとHTTPの最終接続は09・11へ引き継ぐ。

## Verification

- 新規DB migration、Repositoryのrollback、UNIQUE/FK/CAS競合を統合テストする。
- 二重起動、設定不正、root不在・権限不足、終了シグナルをテストする。

検証コマンドの未指定部分は実装したテスト名に合わせて具体化し、実行結果・環境をこのファイルへ追記する。上記は予定であり、現在の合格結果ではない。

## Acceptance criteria

データが再起動後も保持され、コピー中にDB書き込みtransactionを開き続けない構成である。

## Handoff notes

- 前提タスクの成果と検証記録を確認してから着手する。未解決の設計変更は該当承認ゲートへ戻す。
- 予定領域外の変更、共有Cargo設定・router・migrationへの変更が必要なら統合担当へ集約し、並行タスクの所有範囲を侵さない。
- UI変更時はdesign.mdを全文読み、依存関係・連携を変更したらMermaid図も更新する。
- 完了時にStatusをDoneへ更新し、変更ファイル、検証結果、未解決事項を記録する。購入音源・購入者情報はコミットしない。
