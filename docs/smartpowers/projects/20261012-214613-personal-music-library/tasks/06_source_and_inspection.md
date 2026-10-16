# 06: 安全な走査と音源解析

Status: Planned
Project: 20261012-214613-personal-music-library
Requirements: REQ-004, REQ-005, REQ-012, NFR-006, NFR-010
Depends on: 02, 04
Parallel group: B

## Objective

設定rootだけを読み、対応する購入AACから検証済み情報を取得する。

## Design references

MODULE_DESIGN §6・7、02の検証結果。プロジェクトの承認済みPRD・DESIGN_DOC・MODULE_DESIGNを前提とする。

## Scope

### Included

以下の実装手順と検証を含む。

### Excluded

ファイルコピー・DB更新・自動予約。

## Expected code areas

crates/server/src/source/、inspection/、専用fixture・テスト

## Implementation steps

1. descriptor相対・no-followの列挙とread-onlyハンドル、fingerprintを実装する。
2. 02で選んだパーサーを境界内に実装し、AAC・保護構造・購入情報・サンプル範囲を検証する。
3. 各解析上限、音声digest、許可メタデータ、parser_versionを実装する。
4. Deferred/Rejectedの理由を契約codeへ変換し、購入者タグやOS生エラーを外へ出さない。

## Verification

- 外部symlink、途中ディレクトリ差し替え、特殊ファイル、非UTF-8パスをテストする。
- 02の判定fixture、box長・sample offset・上限値をテストする。
- 元内容・サイズ・mtimeが変わらないことを確認する。

検証コマンドの未指定部分は実装したテスト名に合わせて具体化し、実行結果・環境をこのファイルへ追記する。上記は予定であり、現在の合格結果ではない。

## Acceptance criteria

保護・破損・判定不能を理由付きで除外し、購入AACをValidatedAudioへ変換できる。

## Handoff notes

- 前提タスクの成果と検証記録を確認してから着手する。未解決の設計変更は該当承認ゲートへ戻す。
- 予定領域外の変更、共有Cargo設定・router・migrationへの変更が必要なら統合担当へ集約し、並行タスクの所有範囲を侵さない。
- UI変更時はdesign.mdを全文読み、依存関係・連携を変更したらMermaid図も更新する。
- 完了時にStatusをDoneへ更新し、変更ファイル、検証結果、未解決事項を記録する。購入音源・購入者情報はコミットしない。
