# Rustune

iTunes Storeで購入した音楽を、自分のライブラリで管理・再生するRust/WASMアプリ。

現在は開発基盤の実装段階です。購入曲の取り込み・検索・再生機能は未実装です。

開発基盤のビルド・Lint・接続確認は完了しています。先行検証は[音源解析実験](experiments/audio/README.md)と[ブラウザ検証](experiments/browser/README.md)を参照してください。合成データによるテストは実施済みですが、購入曲の識別とネイティブChrome/OS連携の受け入れは未完了です。

## 開発環境

- macOS（ミュージックアプリと同じMac）
- Rustup。`rust-toolchain.toml`でRust 1.99.1、Clippy、rustfmt、WASMターゲットを固定
- Trunk 0.21.14

```sh
rustup show
cargo install trunk --version 0.21.14 --locked
```

Rustの実行ファイルが見つからない場合は、Rustupが案内するcargoの環境設定を読み込んでください。

## 起動

リポジトリルートから：

```sh
cd crates/web
trunk build --release --locked
cd ../..
cargo run --locked -p rustune-server
```

`http://localhost:3867`を開き、「接続を確認」を押すとRustサーバーとの通信を確認できます。音源やDBは必要ありません。サーバーは`127.0.0.1`だけで待ち受けます。

UI開発では別ターミナルで`cd crates/web && trunk serve`を実行し、`http://localhost:8080`を開きます。`/api/`は3867のバックエンドへ転送します。

開発段階の設定：

| 環境変数 | 既定値 | 用途 |
| --- | --- | --- |
| RUSTUNE_PORT | 3867 | loopbackのポート。変更時はTrunkのproxyも合わせる |
| RUSTUNE_WEB_DIST | crates/web/dist | 静的資産の保存先。実行ディレクトリ基準 |
| RUST_LOG | rustune_server=info | ログレベル |

TOMLによる製品設定はタスク05で追加します。現在のhealth APIは接続確認用の`/api/v1/health`であり、取り込み・DB復旧のready判定ではありません。

## 品質チェック

```sh
cargo fmt --all -- --check
cargo clippy --locked -p rustune-domain -p rustune-contracts -p rustune-server --all-targets -- -D warnings
cargo clippy --locked -p rustune-web --target wasm32-unknown-unknown -- -D warnings
cargo test --locked -p rustune-domain -p rustune-contracts -p rustune-server
cd crates/web && trunk build --release --locked
```

UIの見た目は[design.md](design.md)、開発ルールは[AGENTS.md](AGENTS.md)、進捗は[実装計画](docs/smartpowers/projects/20261012-214613-personal-music-library/tasks/PLAN.md)を参照します。購入音源・購入者情報はGitへ追加しません。

実MacのChromeで開発基盤を確認する場合は、サーバー起動後に以下を実行します。既存のChromeプロファイルとは分離したheadless環境で検証し、結果画像はGit対象外の`.local/screenshots/`へ保存します。

```sh
npm install --prefix .local/browser --no-audit --no-fund playwright
node scripts/check-foundation.mjs
```

Trunkが`NO_COLOR=1`の解析で失敗する環境では`env -u NO_COLOR trunk build --release --locked`を使ってください。
