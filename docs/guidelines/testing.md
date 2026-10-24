# テスト規約

## 配置

| 種類 | 置き場所 | 用途 |
| --- | --- | --- |
| ユニットテスト | 対象モジュール内の`#[cfg(test)] mod tests`。長くなる場合は隣の`tests.rs`（`inspection/tests.rs`、`source/tests.rs`） | 非公開関数・内部状態の検証 |
| 結合テスト | `crates/<crate>/tests/<領域>.rs`（`domain/tests/invariants.rs`、`contracts/tests/wire.rs`、`server/tests/storage.rs`） | 公開APIだけを使った振る舞いの検証 |
| 結合テストの共通部品 | `crates/<crate>/tests/support/mod.rs` | fixture・一時ディレクトリなど |
| ブラウザ検証 | `crates/web/tests/*.mjs`、`scripts/` | DOM・再生・Media Session |

## 書き方

- テスト名は、保証する性質を英語の`snake_case`の文で書く（`invalid_audio_never_becomes_publishable`、`unknown_api_does_not_fall_back_to_html`）。
- 1つのテストで1つの性質を検証する。境界値は配列にまとめて`for`で回してよい（`invalid_audio_never_becomes_publishable`）。
- テスト用の値を作る関数は小さく名前を付ける（`track(n)`、`entry(n, t)`、`audio(samples)`）。UUIDは`Uuid::from_u128(n)`で決定的に作る。
- テストでは`unwrap()`を使ってよい。エラーを検証する場合は`is_err()`だけで済ませず、可能ならエラーの種類まで`assert_eq!` / `matches!`で確かめる。
- 状態遷移は、許可される遷移と**拒否される遷移の両方**を検証する。
- contractsのDTOは、JSONの往復と、`deny_unknown_fields`・`Patch`の3状態（省略・null・値）を検証する。

## fixtureと実音源

- 音源のfixtureはテスト内で合成する（`mp4_track::Mp4Writer`で組み立てる`inspection/tests.rs`の`fixture`）。購入音源のファイルをリポジトリに置かない。
- 実音源を使う検証は`AGENTS.md`の「この端末での実音源テスト」に従い、`.local/audio-testing.md`がない環境ではスキップする。実音源の内容・パス・購入者情報をテストコードやassertメッセージに書かない。
- ファイルシステムを使うテストは`tempfile`の一時ディレクトリを使い、テスト間で状態を共有しない。

## 検証コマンド

```sh
cargo fmt --all -- --check
mise run fmt-check   # rustfmtに加え、CSS・JSをoxfmtで確認
cargo clippy --locked -p rustune-domain -p rustune-contracts -p rustune-server --all-targets -- -D warnings
cargo clippy --locked -p rustune-web --target wasm32-unknown-unknown -- -D warnings
cargo test --locked -p rustune-domain -p rustune-contracts -p rustune-server
(cd crates/web && trunk build --release --locked)
```

CI（`.github/workflows/`）も同じ検査を実行する。`cargo test`だけでは再生やDOM操作を検証したことにならない（`MODULE_DESIGN.md` §16）。UIや再生を変えた場合は、ブラウザでの検証結果をタスク記録に残す。
