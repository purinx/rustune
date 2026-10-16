# 07: 音源保存とジャケット処理

Status: Planned
Project: 20261012-214613-personal-music-library
Requirements: REQ-001, REQ-005, REQ-012, NFR-005, NFR-007
Depends on: 04
Parallel group: B

## Objective

元を変更しない独立コピーと、確定・回収可能な一時保存を実装する。

## Design references

MODULE_DESIGN §8。プロジェクトの承認済みPRD・DESIGN_DOC・MODULE_DESIGNを前提とする。

## Scope

### Included

以下の実装手順と検証を含む。

### Excluded

DB transaction、ジョブ管理、外部画像取得。

## Expected code areas

crates/server/src/storage/、artwork/、専用I/O・画像テスト

## Implementation steps

1. UUIDベースの保存先、staging、固定bufferコピー、digest、元ファイル変化の検査を実装する。
2. 予定サイズと空き容量下限、コピー中上限、sync/rename/親ディレクトリ同期を実装する。
3. 管理下ファイルだけの削除とquarantine移動を実装する。
4. 埋め込み画像の制限付きデコード、サムネイル、壊れた画像のフォールバックを実装する。

## Verification

- コピー・sync・rename・削除の障害注入、容量競合、元ファイル変更をテストする。
- JPEG/PNG、画像なし、巨大寸法・圧縮画像・破損画像を検証する。

検証コマンドの未指定部分は実装したテスト名に合わせて具体化し、実行結果・環境をこのファイルへ追記する。上記は予定であり、現在の合格結果ではない。

## Acceptance criteria

部分コピーが完成音源として扱われず、元の移動後も管理下コピーを利用できる。

## Handoff notes

- 前提タスクの成果と検証記録を確認してから着手する。未解決の設計変更は該当承認ゲートへ戻す。
- 予定領域外の変更、共有Cargo設定・router・migrationへの変更が必要なら統合担当へ集約し、並行タスクの所有範囲を侵さない。
- UI変更時はdesign.mdを全文読み、依存関係・連携を変更したらMermaid図も更新する。
- 完了時にStatusをDoneへ更新し、変更ファイル、検証結果、未解決事項を記録する。購入音源・購入者情報はコミットしない。
