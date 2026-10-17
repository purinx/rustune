# 05: SQLite・設定・起動基盤

Status: In Progress
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

## 検証記録（2026-10-18）

環境：macOS 27.0.1、rustc 1.99.1（rust-toolchain.toml固定）、ブランチ`task05-persistence`（mainへrebase済み）。実音源・購入者情報は使用せず、一時ディレクトリと合成行のみで検証。

| コマンド | 結果 |
| --- | --- |
| `cargo fmt --all -- --check` | 合格 |
| `cargo clippy --locked -p rustune-domain -p rustune-contracts -p rustune-server --all-targets -- -D warnings` | 合格 |
| `cargo test --locked -p rustune-domain -p rustune-contracts -p rustune-server` | 合格（persistence 12件合格・補助2件ignored、他crate既存試験も合格） |
| `cargo clippy --locked -p rustune-web --target wasm32-unknown-unknown -- -D warnings` | 合格 |
| `cargo test --locked -p rustune-server --test persistence`を15回連続 | 15回とも合格（フレーク無し） |

`crates/server/tests/persistence.rs`の試験と検証項目の対応：

- 新規DB migration・WAL/FK/busy timeout・再起動後の保持：`migration_pragmas_and_restart_preserve_state`
- UNIQUE/FK/CHECK違反、途中失敗時のrollback：`constraints_and_repository_errors_rollback_without_revision_increment`
- CAS競合：`concurrent_cas_has_one_winner_and_no_lost_updates`、`concurrent_settings_updates_have_one_winner`（範囲外設定値の拒否も確認）
- 取り込み要求の統合・root変更の原子性と実行中409相当：`import_requests_merge_and_settings_root_updates_are_atomic`
- 設定不正・root重なり・シンボリックリンク別名：`runtime_settings_reject_unknown_invalid_values_and_overlapping_roots`
- root不在でもDB起動・ready=false・同一プロセス内二重ロック：`bootstrap_missing_source_stays_not_ready_and_lock_excludes_second_process`
- 権限不足rootでも起動、保存先がファイルなら起動失敗：`unreadable_source_does_not_prevent_database_startup_and_storage_file_is_rejected`
- 別プロセスの二重起動拒否・異常終了後のロック解放：`second_process_is_rejected_and_crash_releases_lock`
- 終了シグナル：`sigterm_is_received_in_an_isolated_child`、協調キャンセル：`shutdown_wakes_all_waiters_and_preexisting_requests_and_drains`、`drain_timeout_reports_unfinished_work`

受け入れ条件：Repositoryの各操作は戻る前にcommit/rollbackを完了し、transactionを呼び出し側へ渡さないため、コピー中に書き込みtransactionを開き続けない構成である。再起動後の保持は上記migration試験で確認。

変更ファイル：`crates/server/migrations/0001_library.sql`、`crates/server/src/{bootstrap.rs,config.rs,lib.rs}`、`crates/server/src/db/{mod.rs,settings.rs,README.md}`、`crates/server/tests/persistence.rs`。

未解決・引き継ぎ：

- recovery・HTTP listener・ワーカーへの最終接続、`main.rs`の`bootstrap`利用はタスク09・11で行う（`main.rs`は未変更）。
- port占有の検出はlistener接続時（タスク11）に試験する。
- `bootstrap`は新規DB用の既定root（`initial_source`）も保存先との重なりを事前検証する。既存DBで別rootを保存済みでも、既定rootが保存先と重なる構成では起動を拒否する（保存先作成前に取り込み元を変更しないための安全側の判断）。
- 業務用Repository操作（reserve_track・publish_track・削除・メタデータ・並べ替え等）はタスク09・10で同じpool・スキーマに追加する。
