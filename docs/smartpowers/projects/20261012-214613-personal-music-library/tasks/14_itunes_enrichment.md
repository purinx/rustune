# 14: iTunesメタデータ・ジャケット補完

Status: Planned
Project: 20261012-214613-personal-music-library
Requirements: REQ-013, NFR-007, NFR-010
Depends on: 13
Parallel group: C

## Objective

ユーザーの明示選択でのみ補完し、既存データを失わないようにする。

## Design references

MODULE_DESIGN §13、design.md全文。プロジェクトの承認済みPRD・DESIGN_DOC・MODULE_DESIGNを前提とする。

## Scope

### Included

以下の実装手順と検証を含む。

### Excluded

高解像度保証、URL書換え、試聴・フル尺取得。

## Expected code areas

crates/server/src/enrichment/、専用ルート定義、crates/web/src/pages/enrichment/、補完テスト

## Implementation steps

1. 利用条件と出典・ストアリンク表示の成立性を確認する。満たせなければShould延期をレビューする。
2. 3.1秒間隔、429/timeout、候補10分/100件・待ち行列上限、公開DTO変換を実装する。
3. 許可host/IP/redirectを検証する画像fetcherと低解像度画像適用を実装する。
4. 候補選択・項目選択・手動値解除確認・revision付き適用をUIへ接続する。previewUrlは扱わない。

## Verification

- 候補期限切れ、429、画像なし/破損、timeout、任意URL、revision競合をテストする。
- 失敗時の既存値保持と画像項目を外した再適用を確認する。

検証コマンドの未指定部分は実装したテスト名に合わせて具体化し、実行結果・環境をこのファイルへ追記する。上記は予定であり、現在の合格結果ではない。

## Acceptance criteria

返された60/100px等の画像を利用でき、選択なしでは情報を変更しない。

## Handoff notes

- 前提タスクの成果と検証記録を確認してから着手する。未解決の設計変更は該当承認ゲートへ戻す。
- 予定領域外の変更、共有Cargo設定・router・migrationへの変更が必要なら統合担当へ集約し、並行タスクの所有範囲を侵さない。
- UI変更時はdesign.mdを全文読み、依存関係・連携を変更したらMermaid図も更新する。
- 完了時にStatusをDoneへ更新し、変更ファイル、検証結果、未解決事項を記録する。購入音源・購入者情報はコミットしない。
