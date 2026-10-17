# 検証記録

## 2026-10-18 再開

タスク03は**未完了**。先に作成した独立した検証ハーネスをworktreeへ移し、既存の指定実音源対応を維持した。音源自体・端末固有のパス・購入者タグはGitへ含めない。

実行可能な検証と人の操作が必要な検証を区別する。Chrome headlessでの成功は音声の聴取、ネイティブインストール、OSからの操作配送の成功ではない。

### 自動検証

環境：macOS 27.0.1 (26A434)、Node v26.9.0、Chrome 155.0.8059.80。

- `node experiments/browser/generate-fixture.mjs`：成功。12秒AAC、123530 bytes、トップレベルbox順`ftyp/free/mdat/moov`。
- `node --test experiments/browser/server.test.mjs`：2/2成功。HTTP 200/206/416、HEAD、単一Range・suffix・open-ended・multi-range fallback。
- `PLAYWRIGHT_PATH=<既存Playwrightの絶対パス> node experiments/browser/browser-check.mjs`：成功。合成音源のduration 12.051156秒、HTTP 206、再生・6秒シーク後の再生継続・一時停止・前後・Media Session登録/解除、390px/1280pxで横溢れなし、pageerrorなし。
- 同コマンドへ`.local/audio-testing.md`の指定ファイルを`RUSTUNE_TEST_AUDIO`として渡した端末内検証：成功。duration 249.637732秒、HTTP 206、上記と同じ再生・シーク等の操作成功、pageerrorなし。読み取り専用でコピーなし、同一loopback origin以外のリクエストを遮断。音源ファイル名と購入者情報はログ・この記録に含めない。
- `node --check experiments/browser/manual-check.mjs`：成功。常駐ブラウザの自動起動は行っていない。

### 実機の残件

- 内蔵ブラウザ接続を再試行したが、環境メタデータの`sandboxPolicy`欠落で起動できなかった。
- `/Applications`にChromeとSafariは存在。Firefoxは存在しない。追加インストール・設定変更は行っていない。
- Safariに付属するsafaridriverは27.0.1。Remote Automationを有効化せず、Safariの基本再生は未実施。
- OSの操作能力を確認するSystem Eventsの読み取りは応答せず中断。OSの権限や設定は変更していない。
- Native Chromeのインストール・名称・アイコン・独立ウィンドウ・Dock再開・外部リンク、実メディアキー・Control Center・背景操作、Safari/Firefox再生・聴音は未実施。

`manual-check.mjs`は人が明示して起動する専用ランナー。loopbackサーバーと一時Chromeプロファイルを用い、Ctrl+Cで自身が作ったプロファイル・サーバー・ブラウザを終了する。既存の個人プロファイルを探索しない。インストールしたOSアプリの削除は利用者の手順であり、ランナーは自動実行しない。

ユーザーがChromeで手動確認を行うと申し出たため、合成音源だけのサーバーを`http://127.0.0.1:4317`で起動した。利用者自身のChromeでアクセスできる。常駐ブラウザランナーは起動せず、Chromeのプロフィールや設定を自動変更していない。インストール等の結果は回答待ち。

## 2026-10-18 最小Manifestへ変更（ユーザー承認済み）

利用者は4317ページをChromeで開けたが、Spotifyのようなアドレスバーのinstallアイコンを期待していた。ページ表示成功のみ確定であり、インストール・独立ウィンドウ成功とは扱わない。

Manifestなしの方針から、インストール情報だけの最小Manifestへ変更することをユーザーが明示承認。name/short_nameをRustune、id/start_url/scopeを`/`、displayをstandalone、背景/テーマをCSSの`--bg`と一致させ、自作SVG由来の192/512pxアイコンを追加した。Service Workerとオフラインは追加していない。

- HTTPテストは3/3成功。追加テストでManifestの配信Content-Type・必須情報・CSS背景色一致・配信PNGの実寸を確認。
- Chrome 155.0.8059.80のCDP `Page.getAppManifest`はerrorsなし、`Page.getInstallabilityErrors`はinstallabilityErrorsなし。検査は一時的な通常プロファイルを使い、終了後に削除した。incognitoでは`in-incognito`となるため静的検査に使わない。
- Service Worker登録0、合成AACの再生・6秒シーク後の継続・停止・前後・Media Session解除、390px/1280pxの横溢れなし、pageerrorなしを回帰確認。
- 指定実音源も同条件で回帰成功。duration 249.637732秒、HTTP206、installabilityErrorsなし、pageerrorなし。実音源・購入者情報は保存・外部送信しない。
- 4317サーバーを同URLで再起動し、ユーザーの再読み込み・インストール再確認待ち。installアイコン表示と実インストールをCDP静的検査から断定しない。
