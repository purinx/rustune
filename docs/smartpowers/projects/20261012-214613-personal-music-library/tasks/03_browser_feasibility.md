# 03: Chromeインストール・再生・OS操作の検証

Status: Planned
Project: 20261012-214613-personal-music-library
Requirements: REQ-007, REQ-015, REQ-016
Depends on: 01
Parallel group: A

## Objective

ManifestなしのインストールとAAC再生・Media Sessionが同じMacで成立することを先行確認する。

## Design references

DESIGN_DOC Testing strategy、MODULE_DESIGN §12・14・15。プロジェクトの承認済みPRD・DESIGN_DOC・MODULE_DESIGNを前提とする。

## Scope

### Included

以下の実装手順と検証を含む。

### Excluded

PWA、Service Worker、Tauri導入、正式なPlayerController実装。

## Expected code areas

experiments/browser/、専用検証ページ・サーバー、プロジェクト配下のブラウザ検証記録

## Implementation steps

1. 専用の最小検証ページとRange配信を用意し、製品UIに混ぜない。
2. Chromeで名前・アイコン・独立ウィンドウ・外部リンク・再表示を確認する。
3. AAC再生、シーク、末尾moov、メディアキー、コントロールセンター、背景ウィンドウ操作を確認する。
4. Safari/Firefoxの基本再生も確認し、OSとブラウザ版、手動再現手順を記録する。不成立なら設計へ戻す。

## Verification

- 実MacのChromeへインストールして受け入れ項目をチェックする。
- 各ブラウザで再生・シークし、HTTPの200/206/416を確認する。
- メディアキーから再生・停止・前後移動を実操作する。

検証コマンドの未指定部分は実装したテスト名に合わせて具体化し、実行結果・環境をこのファイルへ追記する。上記は予定であり、現在の合格結果ではない。

## Acceptance criteria

必須ブラウザ挙動が確認済みであり、未実施と不合格を区別した記録がある。

## Handoff notes

- 前提タスクの成果と検証記録を確認してから着手する。未解決の設計変更は該当承認ゲートへ戻す。
- 予定領域外の変更、共有Cargo設定・router・migrationへの変更が必要なら統合担当へ集約し、並行タスクの所有範囲を侵さない。
- UI変更時はdesign.mdを全文読み、依存関係・連携を変更したらMermaid図も更新する。
- 完了時にStatusをDoneへ更新し、変更ファイル、検証結果、未解決事項を記録する。購入音源・購入者情報はコミットしない。
