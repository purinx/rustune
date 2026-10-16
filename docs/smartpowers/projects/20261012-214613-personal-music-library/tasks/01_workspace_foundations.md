# 01: 開発基盤とCI

Status: Done
Project: 20261012-214613-personal-music-library
Requirements: NFR-001, NFR-002
Depends on: None
Parallel group: None

## Objective

nativeサーバーとWASM UIを別ターゲットでビルドできる基盤を作る。

## Design references

MODULE_DESIGN §1・16。プロジェクトの承認済みPRD・DESIGN_DOC・MODULE_DESIGNを前提とする。

## Scope

### Included

以下の実装手順と検証を含む。

### Excluded

機能実装、Musicフォルダの走査、LaunchAgentの有効化。

## Expected code areas

Cargo.toml、Cargo.lock、rust-toolchain.toml、各crateのCargo.tomlと最小entrypoint、Trunk設定、.github/workflows/、.gitignore

## Implementation steps

1. rustune-domain/contracts/server/webのworkspaceと依存方向を設定する。互換な安定版とRust toolchainを固定する。
2. Trunkの開発・release build、設定例、起動方法を用意する。
3. rustfmt、native/WASM別Clippy、nativeテスト、WASMビルドをCIへ設定する。購入音源・個人情報・targetを追跡しない。

## Verification

- cargo fmt --all -- --check
- cargo clippy -p rustune-domain -p rustune-contracts -p rustune-server --all-targets -- -D warnings
- cargo clippy -p rustune-web --target wasm32-unknown-unknown -- -D warnings
- cargo test -p rustune-domain -p rustune-contracts -p rustune-server
- cd crates/web && trunk build --release

検証コマンドの未指定部分は実装したテスト名に合わせて具体化し、実行結果・環境をこのファイルへ追記する。上記は予定であり、現在の合格結果ではない。

## Acceptance criteria

空のDBや音源がなくてもビルドが通り、サーバー起動とWASMの最小画面表示ができる。

## Handoff notes

### 実施結果（2026-10-16）

- Rust 1.99.1、Trunk 0.21.14、4クレートworkspaceとCargo.lockを追加。
- native/WASM別Clippy、cargo fmt、nativeテスト2件、Trunk release buildに合格。
- 実Macの独立headless Chromeで1280px・390pxのWASM mount、HTTP接続、画像配信、横overflowなし、pageerrorなしを確認。再現コマンドはREADMEとscripts/check-foundation.mjs。
- サーバーは127.0.0.1:3867。ライブラリ・取り込み・再生は未実装。現在の画面は開発基盤の接続確認。
- アプリ内ブラウザ接続はsandboxPolicyメタデータ不足で利用できず、独立Chromeへ切り替えた。OSメディア操作・Chromeインストールの検証を代替したとは扱わない。

- 前提タスクの成果と検証記録を確認してから着手する。未解決の設計変更は該当承認ゲートへ戻す。
- 予定領域外の変更、共有Cargo設定・router・migrationへの変更が必要なら統合担当へ集約し、並行タスクの所有範囲を侵さない。
- UI変更時はdesign.mdを全文読み、依存関係・連携を変更したらMermaid図も更新する。
- 完了時にStatusをDoneへ更新し、変更ファイル、検証結果、未解決事項を記録する。購入音源・購入者情報はコミットしない。
