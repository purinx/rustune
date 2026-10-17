# 永続化と起動の境界

`Repository::open`はSQLiteを作成・migrationし、WAL、外部キー有効、synchronous=FULL、
busy timeout 5秒、接続数上限4で開く。SQLは動的クエリであり、ビルド時のDBやSQLxの
offlineキャッシュは不要。初期スキーマは`migrations/0001_library.sql`が所有する。
リリース済みのmigrationは書き換えず、後続のmigrationを追加する。

内部の行データや取り込み元パスのOSバイト列はAPIレスポンスへSerializeしない。
Repositoryへ渡す時刻はサーバーが生成するUTCのRFC 3339文字列とする。公開revisionは
u64で、SQLiteには非負のi64として保存し、オーバーフロー時は折り返さず拒否する。

提供するRepository操作は、戻る前に必ずcommitまたはrollbackを完了する。取り込み予約は
並行したスキャン要求を1件へまとめる。設定のroot変更は、現在値を読む前に書き込み
ロックを取得し、revisionのCASと実行中importが無いことを確認したうえで、過去の
source rootを保持する。自動取り込みの無効化は既存jobに影響しない。プレイリストの
変更と表示に影響する設定変更は、同じtransaction内でlibrary revisionを増やす。CASの
失敗やrevision加算の失敗では、すべての書き込みをrollbackする。

取り込みの確定・復旧とライブラリサービスの操作はタスク09/10で追加する。trackの予約・
確定、識別子の対応付け、メタデータ更新、プレイリスト項目の並べ替え、お気に入り、
削除・除外のtransactionも同じpool・スキーマを使う。それらのDB transactionを
コピー、rename、画像処理、外部通信の間に保持し続けてはならない。`pool()`はSQLを扱う
サービスとテストのためのものであり、ファイル操作アダプタへtransactionを漏らす用途ではない。

`config::load_config(text, home)`はTOMLの運用設定だけを検証する。対象はloopbackの
bind先、0以外のport、データ保存先、Web資産、ログフィルタ。取り込みroot、自動取り込み、
間隔、ファイルサイズ・空き容量の上限、storefrontはDB設定だけが所有する。既定値は
新規DBにだけ挿入する。相対パスや親ディレクトリへの移動を含むパスは拒否する。rootの
重なりはパスの構成要素単位で比較し、存在する祖先を実パスへ解決するため別名も検出する。
取り込み元rootの不在・読み取り不可は状態として報告し、起動失敗にはしない。実際の取り込み
時のno-followなdescriptorアクセスと、識別・包含関係の再検証はタスク06が担う。

`bootstrap(config, initial_source)`は非公開の保存先を作成し、コンテキストが生きている
間は非ブロッキングのadvisoryプロセスロックを保持してから、DBを開いてmigrationし、
永続化された設定を読み込む。ロックファイルは意図的に削除しない。クラッシュ後でも
OSがロックを解放するため、残ったファイルは無害である。この関数はMusicのファイル、
LaunchAgent、ネットワークlistener、ワーカーを作成しない。`ready()`は、タスク09が
実際のrecovery後に`mark_recovered()`を呼ぶまでfalseのままとなる。アプリケーションの
エントリポイントへの接続はタスク05では意図的に行わない。

`Shutdown`は全ワーカーへ協調キャンセルを要求し、最大10秒（`SHUTDOWN_DRAIN_TIMEOUT`）
完了を待つ。HTTP・オーケストレーター側の統合では、まず新規予約とschedulerの定期実行を
止め、ワーカーのキャンセルを要求して完了を待ち、その後poolを閉じてプロセスロックを解放する。
タイムアウトは次回起動時のrecoveryで処理すべき作業が残っていることを意味し、確定処理が
成功したことは意味しない。`ShutdownSignal::install()`は準備完了の報告前にSIGTERM/SIGINTの
ハンドラを登録し、`recv()`でいずれかを待つ。`shutdown_signal()`はこの2つをまとめた関数。

```mermaid
sequenceDiagram
    participant Main as 後続のエントリポイント
    participant Config as 実行時TOML
    participant Boot as Bootstrap + ProcessLock
    participant Repo as Repository / SQLite
    participant Recovery as タスク09のrecovery
    Main->>Config: load_config(text, home)
    Main->>Boot: bootstrap(config, initial_source)
    Boot->>Boot: 非公開保存先の準備 + 排他ロック
    Boot->>Repo: WAL pool + migration + 永続設定の読み込み
    Boot-->>Main: コンテキスト、ready=false、取り込み元の状態
    Main->>Recovery: listener・ワーカー起動前にrecovery
    Recovery-->>Main: 成功
    Main->>Boot: mark_recovered
    Note over Main,Boot: HTTP・ワーカーの接続はタスク05の範囲外
```

検証には一時ファイルと合成した行データだけを使う。テストはmigration後の再オープン、
制約、transaction途中の失敗によるrollback、並行CAS、並行した取り込み要求、root設定、
保存先・取り込み元の可用性、別プロセスに対するプロセスロック（クラッシュ後の解放を含む）、
drainのタイムアウト、設定更新の並行CAS、独立した子プロセスでのSIGTERM受信を確認する。
購入音源は不要。
