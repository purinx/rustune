# 検証記録

## 2026-10-18 再開

再開時点ではタスク03は未完了だった（現在の結果は末尾参照）。先に作成した独立した検証ハーネスをworktreeへ移し、既存の指定実音源対応を維持した。音源自体・端末固有のパス・購入者タグはGitへ含めない。

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

### ユーザーによる実Chrome確認

ユーザーから「問題なく追加、アップデートできたよ！」と回答があった。最小Manifest反映後の実Chromeでのアプリ追加・更新は成功として記録する。

installアイコンの表示位置・名称やアイコンの見え方・独立ウィンドウの詳細、Dockからの再起動、外部リンクの引き渡し、聴音、物理メディアキー・Control Center・背景操作、Safari/Firefoxについては、この回答だけで成功とは扱わない。これらの残件確認のため4317の検証サーバーを維持している。

追加回答で、Dockから開き直せたこと、音が出て6秒へのシーク後も再生が続いたことを確認した。この時点で配信していたのは合成AACであり、ユーザー指定の楽曲ではない。Dockからの再表示、合成AACの聴音・シーク継続は手動成功として記録する。

続いて、ユーザーが指定した実音源の聴音を確認するため、`.local/audio-testing.md`で指定済みの音源を同じ4317サーバーへ切り替えた。元ファイルを読み取り専用で扱い、音源をコピーせずloopbackだけへ配信する。この専用起動はCSP `default-src 'self'`を付け、ファイル名・購入者情報をログに出さない。画面とMedia Sessionには引き続きダミー名を使い、購入者タグや実曲名を取得しない。ユーザーによる実音源の聴音・シーク結果は回答待ち。

### 現時点の実機結果

指定曲へ切替後、ユーザーが問題なく再生できると報告した。続いて「キーボードのメディアキーもコントロールセンターからも操作できてるよ！」「開けたよ！」との回答を得た。直前の確認依頼は、別ウィンドウを前面にした状態での再生・停止・前後操作、および通常Chromeへ外部リンクを開いてRustuneのウィンドウを置換しない確認である。この依頼文脈で操作成功として記録する。曲名・位置などのOS表示の詳細やセッション解除後の表示消去は個別確認していない。

前後操作は同じ音源を使うダミーキューの切替であり、正式プレイヤー・別曲への切替まで実証したものではない。Safari/Firefoxの再生・停止・シークは引き続き未実施。これらの追加確認に備えて指定実音源の4317サーバーを維持している。

### 最終の受け入れ対象と結果

ユーザーが「Safari/Firefox要件指定つもりない、削除していい」と明示したため、対応ブラウザをChromeだけへ変更した。Safari/Firefoxは検証対象外とし、追加試験・追加導入は行わない。前節までの未実施記録は当時の履歴であり、現在の未完了理由ではない。

Chromeで必要なインストール・更新、Dockからの再表示、合成AACの聴音・6秒シーク継続、指定曲の聴音、背景でのメディアキーとControl Centerからの再生・停止・前後操作、通常Chromeへの外部リンクはユーザー確認で成功した。HTTP/Manifest/CDP静的検査と合成/指定実音源の自動回帰結果も併せて、変更後のタスク03受け入れに必要な検証は満たしている。

OSの曲名・位置表示の細かな見え方とセッション解除後の残留表示は個別確認していない（JavaScriptの解除は自動検証済み）。正式PlayerController、別曲を使う正式キュー、製品のRust/WASMとの接続は後続タスクの対象。この検証の成功範囲を広げて解釈しない。

ユーザーが引き続き聴音できるよう、指定実音源のloopback4317サーバーを維持している。タスク状態・要件文書の更新は統合担当が行う。
