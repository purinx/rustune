# 04: ドメインモデルとAPI DTO

Status: Done
Project: 20261012-214613-personal-music-library
Requirements: REQ-004, REQ-005, REQ-008, NFR-001, NFR-002, NFR-010
Depends on: 02
Parallel group: B

## Objective

共有の型・不変条件・通信仕様を、DBやブラウザの都合から分離する。

## Design references

MODULE_DESIGN §1–3。プロジェクトの承認済みPRD・DESIGN_DOC・MODULE_DESIGNを前提とする。

## Scope

### Included

以下の実装手順と検証を含む。

### Excluded

SQL、HTTP handler、実ファイルI/O。

## Expected code areas

crates/domain/src/、crates/contracts/src/、両crateのテスト

## Implementation steps

1. TrackId、楽曲状態、ValidatedAudioの検証コンストラクタ、識別キーを実装する。
2. メタデータ優先順位、検索正規化、取り込み状態、重複entryを扱うキューを純粋関数として実装する。
3. 公開DTO、revision、ページング、Keep/Set/ClearのPATCH、エラーcodeを定義する。
4. domain型と通信DTOの変換境界を明文化し、購入者タグを公開型に含めない。

## Verification

- cargo test -p rustune-domain -p rustune-contracts
- 両crateのwasm32-unknown-unknownでのcargo check。
- PATCH省略/null、識別衝突、Unicode、キュー空・末尾・削除をテストする。

検証コマンドの未指定部分は実装したテスト名に合わせて具体化し、実行結果・環境をこのファイルへ追記する。上記は予定であり、現在の合格結果ではない。

## Acceptance criteria

設計した不正状態を生成・遷移で拒否し、DTOの往復とnative/WASMビルドが通る。

## Handoff notes

### 2026-10-18 完了

- subagent成果`cd4cb16`をmainへ統合。domainの型・不変条件・識別衝突・メタデータ優先・状態遷移・キュー、contractsのDTO・PATCH3状態・公開エラー・入力検証を実装した。変換境界は両crate READMEへ記録。
- 統合後にdomain7件・contracts6件・既存server2件のテスト成功。native Clippy、domain/contractsのwasm32 check、全体rustfmtに成功。subagentはWASM Clippyも成功。
- HTTP・SQL・ブラウザ・実音源の処理は含めない。05/06/07は公開型を参照でき、08は引き続き独立したUI基盤として進める。

- 2026-10-18 by user：「task 04もsubagentで進めて」により03と並行して開始する。02の音源・識別結果を前提とし、ブラウザ非依存のdomain/contractsは03の手動受け入れを待たない。03は08の依存として維持する。

- 前提タスクの成果と検証記録を確認してから着手する。未解決の設計変更は該当承認ゲートへ戻す。
- 予定領域外の変更、共有Cargo設定・router・migrationへの変更が必要なら統合担当へ集約し、並行タスクの所有範囲を侵さない。
- UI変更時はdesign.mdを全文読み、依存関係・連携を変更したらMermaid図も更新する。
- 完了時にStatusをDoneへ更新し、変更ファイル、検証結果、未解決事項を記録する。購入音源・購入者情報はコミットしない。
