# Rust コーディング規約

## 1. 言語・コメント

- doc comment（`//!`・`///`）、コード内コメント、README、コミットメッセージ、PRは日本語で書く。識別子・ログメッセージ・`Display`の文字列は英語でよい。
  - `main`の`domain`・`contracts`には英語のdoc commentが残っている。変更時に日本語へ寄せる。
- 各モジュールの先頭に`//!`で**契約**を書く。何を保証し、何を扱わないか（「DBトランザクションは扱わない（タスク05・09・10）」など）を明記する。例：`server::storage`、`server::source`、`server::inspection`のモジュールdoc。
- `///`は呼び出し側が誤用しうる前提だけに付ける（例：`Track::transition`の「Caller must check persisted exclusions」）。getterや自明な関数には付けない。
- コメントは「なぜ」を書く。処理の言い換えは書かない。
- crate単位の不変条件と依存の説明は、各crateの`README.md`にMermaid図付きで置く（`crates/domain/README.md`、`crates/contracts/README.md`）。

## 2. 命名

- 型は`UpperCamelCase`、関数・変数は`snake_case`、定数は`SCREAMING_SNAKE_CASE`。
- getterに`get_`を付けない（`id()`、`state()`、`size_bytes()`）。
- 中身を借用で返すときは`as_*`（`as_str`、`as_uuid`）、値で返すときは`value()`、変換で新しい値を作るときは`to_*`（`to_hex`）、検証付きで作るときは`new` / `from_*`（`from_matched_tag_pattern`、`from_hex`）。
- 状態は`*State`、状態を変えるイベントは`*Event`、遷移関数は`transition` / `transition_*`。
- 副作用の指示を返す型は`*Effect`（`QueueEffect`）。
- エラー型は、domainでは`Invalid*`（`InvalidAudio`、`InvalidMetadata`、`InvalidQueue`）、serverでは`<モジュール名>Error`（`StorageError`、`SourceError`、`DbError`）。
- 単位はフィールド名の接尾辞で示す：`_ms`、`_bytes`、`_seconds`。
- 版付きの方式名は定数にする（`AUDIO_DIGEST_SCHEME = "rustune-aac-probe-v1"`、`PARSER_VERSION`）。方式を変えたら版を上げる。
- テスト内の短い変数（`t`、`d`、`m`）は許容する。本体コードでは`i`・`e`・`m`のような1文字の引数名を避ける（`metadata::text`の`i, e, m`は直す対象）。

## 3. 型設計

- **不変条件はnewtypeで守る。** フィールドを非公開にし、検証付きの`new`が`Result`を返す（`Text`、`TrackNumber`、`TrackId`、`ValidatedAudio`）。検証済みの値はgetterだけで公開する。
- フィールドを`pub`にしてよいのは、どの値の組み合わせも正しいデータ（`SampleRange`、`QueueEntry`、`Metadata`）と、contractsのDTOだけ。
- 「検証済み」「未確定」の段階を型で分ける（`Staging` → `StagedFile` → 確定、`OpenedSource`）。段階を飛ばすAPIを作らない。
- 状態遷移は`match (state, event)`の全列挙で書き、許可しない組み合わせを`_ => Err(..)`にまとめる（`transition_job`、`transition_item`、`Track::transition`）。
- 数値計算は、外部入力やサイズが絡む場合`checked_add`などで溢れを検査する（`Progress::new`、`ValidatedAudio::new`、`Pagination::validate`）。
- 呼び出し側でIDを生成し、domain・contractsは乱数・時計を呼ばない。nil UUIDは拒否する。
- 引数が多い関数で`#[allow(clippy::too_many_arguments)]`を使う前に、引数をまとめる構造体を検討する。やむを得ず使う場合は理由をコメントに書く。

## 4. エラー処理

- エラー型は手書きのenum（またはunit struct）で定義する。`thiserror`・`anyhow`は導入していない。新しく導入する場合は依存追加として別コミットで提案する。
- serverのエラー型は次をそろえる：
  - `impl Display`：内部向けの短い英語。OS・parserの生メッセージを含めない。
  - `impl std::error::Error`：原因を保持する場合は`source()`を実装する。
  - 下位エラーからの`From`（`From<sqlx::Error> for DbError`など）。
  - 公開用の対応付け：`error_code() -> ErrorCode`（または取り込み理由の`reason() -> &'static str`）と`retryable() -> bool`。
- 利用者に見せる文言は`contracts::error::ErrorCode::message()`の固定日本語文だけを使う。HTTPステータスとの対応はserverが持つ。
- domainのエラーは理由を持たないunit struct（`InvalidAudio`）。理由の区別が必要になったら、domainではなく呼び出し側（server）のエラーで区別する。
- `main`の`Result<(), Box<dyn Error>>`は`main.rs`だけで使う。ライブラリ側の関数は具体的なエラー型を返す。
- エラーを`let _ =`で捨てるのは、失敗しても回復手段も通知先もない場合だけ（例：`focus_with_options`）。捨てる理由が自明でなければコメントを書く。

## 5. panic

- 本体コードで`unwrap()`を使わない。
- `expect("…")`は、静的に成り立つ不変条件にだけ使い、その不変条件を文字列に書く（`expect("static valid text")`、`expect("register SIGTERM handler")`）。
- 添字アクセス（`entries[index]`）は、直前で範囲を確認した場合だけ使う。それ以外は`get()`を使う。
- テストコードでは`unwrap()`を許容する。

## 6. import・ファイル構成

- import順は「`std` → 外部crate → workspace crate（`rustune_*`） → `crate` / `super`」。グループ間に空行を入れる（`server::storage`の書き方）。`main`の`domain`には`crate::`を先に書いたファイルがあるので、変更時に並べ替える。
- ワイルドカードimportはテストと、`match`内で使う`use Enum::*`だけに使う（`Track::transition`）。
- 公開APIの再exportは`mod.rs`（または`lib.rs`）の`pub use`で行い、内部のファイル構成を呼び出し側に見せない（`pub use io::{OsIo, StorageIo};`）。
- ファイル・関数の大きさの基準は[responsibility-separation.md](responsibility-separation.md#4-サイズの目安)を参照する。

## 7. 定数・数値

- 意味のある数値は名前付き定数にする（`MIB`、`GIB`、`STAGING_DIR`）。
- 同じ上限値を複数のcrateで使う場合も、各crateで定数にして名前をそろえる。例：メタデータ文字数の上限512は`domain::metadata::Text::new`と`contracts`の`validate_text(…, 512)`の両方にあるので、`MAX_TEXT_CHARS`などの定数にする。
- `contracts::settings::SettingsPatch::validate`の`1_048_576`・`1_073_741_824`などのバイト値は、`MIB`・`GIB`の定数式で書く。
- 既定値は`Default`実装に置き、どの設計書の値かをコメントで示す（`CopyLimits`の「MODULE_DESIGN §4の`max_file`・`reserve_free`」）。

## 8. プラットフォーム・安全性

- `unsafe`は禁止（workspace lintで`forbid`）。
- 対象はmacOS。Unix専用APIは`std::os::unix`・`rustix`で使い、`main.rs`のシグナル処理のように`#[cfg(unix)]`で分ける。
- ファイルパスはバイト列として扱い、非UTF-8の名前を損失変換しない（`server::source`）。
- 利用者入力（曲名など）をパスの組み立てに使わない。管理下のパスはUUIDから組み立てる（`ManagedFile`）。
- 購入者・アカウント情報はinspectionの外へ出さず、ログにも出さない。
