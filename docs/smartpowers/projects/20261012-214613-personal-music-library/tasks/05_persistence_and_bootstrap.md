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

### 追記：PR #2 レビュー指摘への対応（2026-10-24）

- root重なり判定：解決できる最も深い祖先までcanonicalizeし、残りの成分を足して比較するよう変更（`config::root_relation`、`Separate`/`Overlapping`/`Unverifiable`）。設定変更用の`validate_separate_roots`は分離を確認できない場合に`UnverifiableRoots`で拒否し、起動時の`reject_overlapping_roots`は確認済みの重なりだけを失敗とする。
- web_dist：明示指定・既定値とも`absolute_clean`で検証。既定値はDESIGN_DOCのリリース配置に合わせ`<実行ファイルのディレクトリ>/web`（絶対パス）。開発時はTOMLで絶対パスを明示する（`crates/server/src/db/README.md`に記載）。
- 追加試験：`web_dist_must_be_absolute_and_defaults_next_to_the_binary`、`overlap_through_alias_is_detected_even_below_unreadable_directory`、`unverifiable_source_is_not_confirmed_separate_but_startup_continues`。
- 検証：CI相当4コマンドすべて合格（persistence 15件合格・補助2件ignored）。persistence試験10回連続合格。

### 追記：PR #2 再レビュー指摘への対応（2026-10-25）

- 既存DBの保存済み取り込み元を、保存先の準備とmigrationより前に読み取り専用で取り出し（`db::settings::stored_source_path`）、重なっていれば取り込み元の配下へ書き込まずに起動を止める。
- `config::root_relation`：リンク先が未作成のsymlinkを`symlink_metadata`・`read_link`でたどって比較する（`LinkResolver`）。相対リンクの`..`は実パスへ解決してから戻り、循環は`ConfigError::Io`。
- テスト追加：`stored_source_overlap_is_rejected_before_storage_is_prepared`、`overlap_through_symlink_to_missing_target_is_detected`。
- 検証：fmt（rustfmt・oxfmt）、native Clippy（警告をエラー扱い）、native テスト（server persistence 17件成功・2件はサブプロセス用でignored）。
