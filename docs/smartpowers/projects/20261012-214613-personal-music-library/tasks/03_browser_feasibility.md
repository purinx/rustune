# 03: Chromeインストール・再生・OS操作の検証

Status: Done
Project: 20261012-214613-personal-music-library
Requirements: REQ-007, REQ-015, REQ-016
Depends on: 01
Parallel group: A

## Objective

最小ManifestによるChromeのインストール導線とAAC再生・Media Sessionが同じMacで成立することを先行確認する。

## Design references

DESIGN_DOC Testing strategy、MODULE_DESIGN §12・14・15。プロジェクトの承認済みPRD・DESIGN_DOC・MODULE_DESIGNを前提とする。

## Scope

### Included

以下の実装手順と検証を含む。

### Excluded

Service Worker、オフライン再生、Tauri導入、正式なPlayerController実装。

## Expected code areas

experiments/browser/、専用検証ページ・サーバー、プロジェクト配下のブラウザ検証記録

## Implementation steps

1. 専用の最小検証ページとRange配信を用意し、製品UIに混ぜない。
2. 最小Manifestと192px/512pxアイコンを用意し、Chromeのインストール診断・アドレスバーのアイコン・名前・独立ウィンドウ・外部リンク・再表示を確認する。
3. AAC再生、シーク、末尾moov、メディアキー、コントロールセンター、背景ウィンドウ操作を確認する。
4. MacのChromeの基本再生とOS連携について、OS・ブラウザ版、手動再現手順を記録する。不成立なら設計へ戻す。

## Verification

- 実MacのChromeへインストールして受け入れ項目をチェックする。
- MacのChromeで再生・シークし、HTTPの200/206/416を確認する。
- メディアキーから再生・停止・前後移動を実操作する。

検証コマンドの未指定部分は実装したテスト名に合わせて具体化し、実行結果・環境をこのファイルへ追記する。上記は予定であり、現在の合格結果ではない。

## Acceptance criteria

必須ブラウザ挙動が確認済みであり、未実施と不合格を区別した記録がある。

## Handoff notes

### 2026-10-18 完了

- Chromeのみを対象とする最新の受け入れ条件に基づきDoneとする。subagentの最終記録`2b2e4c6`をmainへ統合した。
- HTTPテスト3件、合成/指定実音源の自動再生・シーク・Media Session回帰、Manifest解析・installability診断エラー0、Service Worker登録0を確認済み。rootの統合確認も成功済み。
- 実機ではユーザーが追加・更新、Dock再表示、聴音・シーク継続、指定曲再生、背景メディアキー・コントロールセンター・外部リンクを確認した。
- 08へ最小Manifest/アイコン/外部リンク、12へMedia Session操作を引き継ぐ。正式な製品プレイヤー・別曲キュー・OS表示の詳細は後続受け入れで確認する。詳細は`experiments/browser/VERIFICATION.md`。

### 実装・検証記録

#### 2026-10-18 現在の実機結果（以下の過去の未実施記録を更新）

- ユーザー確認：Chromeへの追加・更新、Dockからの再起動、合成AACの聴音・シーク後の継続、指定実音源の聴音に成功。
- ユーザー確認：別ウィンドウ前面でのキーボードメディアキーとコントロールセンターによる再生・一時停止・前後操作、外部リンクを通常Chromeで開く操作に成功。前後操作は同じ音源を使う検証キューであり、正式キューの保証ではない。
- 2026-10-18のユーザー指示によりSafari/Firefoxの基本操作検証を対象外とした。追加インストールや他ブラウザの合格判定は不要。

- 2026-10-18 実機確認：ユーザーから「問題なく追加、アップデートできた」と回答。Manifest反映後のChromeで追加・更新が成功したことを確認。Dockからの再表示、外部リンク、聴音、実メディアキー・コントロールセンター等はこの回答から合格と推測せず、引き続き確認する。
- 続くユーザー確認でDockからの再表示に成功。検証ページの合成AACは聴音でき、6秒へのシーク後も再生が継続した。指定実音源とは別の合成音源であることを説明し、指定曲での聴音へ切り替える。外部リンク・実OSメディア操作は引き続き回答待ち。

- 2026-10-18 by user：最小Manifestの追加を承認。アドレスバーからのインストール導線を検証する。以下のManifestなしの結果は変更前の記録であり、新条件の合格とは扱わない。
- subagentの検証ハーネス`44163c6`をmainへ統合済み。HTTPテスト2件、Chrome合成AAC回帰とroot rustfmtに成功。ユーザーは検証ページを開けたが、インストール成功はまだ未確認。
- 最小Manifest追加`f236d6c`もmainへ統合。HTTP3件、Chrome 155.0.8059.80の合成AAC再生・シーク・Media Session回帰成功。CDPのManifest解析エラー・インストール診断エラーとも0、Service Worker登録0。subagentは指定実音源の回帰も成功。rootはmainでHTTP・合成AAC・診断を再確認。ユーザーへ同じURLの再読み込みとインストール操作を依頼中。実アイコン表示・OS操作はまだ未確認。

- 2026-10-17: `RUSTUNE_TEST_AUDIO`で端末内の指定ファイルを読み取り専用で使用する自動検証を追加。CIでは指定を拒否し、ブラウザからの通信を同一loopback originに限定。実音源はコピー・コミットしない。
- 指定実音源をChrome 155.0.8059.40 headlessで検証し、再生開始、6秒シーク完了後の再生継続、一時停止、Media Session登録・解除に成功。duration約249.64秒、音源応答206、pageerrorなし。ダミーキューの前後操作も成功。聴音・全曲デコード・実OS操作の保証にはしない。
- 同日の回帰検証：HTTPテスト2/2、合成AACブラウザ検証とも成功。専用テストサーバーは検証後に終了。ユーザーが確認した基盤サーバー（3867）も停止済み。

- `experiments/browser/`に独立した検証ページ、Rangeサーバー、合成AAC生成、自動テスト、実機チェックリストを追加。
- 実行環境：macOS 26.5.2 (25F84)、Node v26.9.0、Chrome 155.0.8059.40。実行日2026-10-16。
- `node experiments/browser/generate-fixture.mjs`成功：合成12秒AAC、123530 bytes、トップレベルbox順`ftyp/free/mdat/moov`。購入音源は使用していない。
- `node --test experiments/browser/server.test.mjs`：2/2成功。HTTP 200/206/416、HEAD bodyなし、Content-Range、suffix/open-ended、multi-rangeの200 fallbackを検証。
- `node experiments/browser/browser-check.mjs`：独立したChrome headlessでAACデコード・再生開始・6秒シーク・一時停止・UIからの前後切り替え、Media Session登録・metadata・解除、390px/1280pxの横溢れなしを確認。音源リクエストは206、pageerrorなし。
- Codex内ブラウザの接続は親担当が試行し、環境メタデータのsandboxPolicy欠落で失敗。既存Chromeの独立した一時プロファイルへフォールバックした。
- **未実施**：ネイティブChromeへのインストール・Dock再表示・インストールウィンドウの外部リンク、実メディアキー・コントロールセンター・背景操作・聴音、Safari/Firefox。手順は`experiments/browser/README.md`。headless成功をこれらの受け入れ成功として扱わない。
- 未実施項目があるためStatusはIn Progressを維持。正式なPlayerController、WASM接続、購入音源への適合はこの検証の成果に含まない。

- 前提タスクの成果と検証記録を確認してから着手する。未解決の設計変更は該当承認ゲートへ戻す。
- 予定領域外の変更、共有Cargo設定・router・migrationへの変更が必要なら統合担当へ集約し、並行タスクの所有範囲を侵さない。
- UI変更時はdesign.mdを全文読み、依存関係・連携を変更したらMermaid図も更新する。
- 完了時にStatusをDoneへ更新し、変更ファイル、検証結果、未解決事項を記録する。購入音源・購入者情報はコミットしない。
