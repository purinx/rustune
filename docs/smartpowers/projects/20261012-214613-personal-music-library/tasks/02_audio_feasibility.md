# 02: 購入音源の識別・重複防止の検証

Status: Planned
Project: 20261012-214613-personal-music-library
Requirements: REQ-004, REQ-012, NFR-010
Depends on: 01
Parallel group: A

## Objective

購入AACを許可し保護音源を除外できる判定規則と、再ダウンロードの同一性判定を実証する。

## Design references

DESIGN_DOC Testing strategy、MODULE_DESIGN §3・7。プロジェクトの承認済みPRD・DESIGN_DOC・MODULE_DESIGNを前提とする。

## Scope

### Included

以下の実装手順と検証を含む。

### Excluded

製品の自動走査・保存、独自パーサーを先に作ること。

## Expected code areas

experiments/audio/、同ディレクトリの検証コード・非機密fixture、プロジェクト配下の音源検証記録

## Implementation steps

1. 使用可能な実購入音源・再ダウンロード版を確認する。生音源や購入者タグをGit・通常ログへ出さない。
2. Rustの形式検証ライブラリを比較し、AAC構造、暗号化、サンプル範囲、購入由来識別の取得を試す。
3. タグ違い・別パス・同一曲再ダウンロードのdigestとストアIDを比較し、採用規則と制限を記録する。
4. 成立しないMust要件があれば後続06を開始せず設計レビューへ戻す。

## Verification

- 購入AAC、DRM、拡張子を変えたDRM、CD AAC、ALAC、途中・破損ファイルの判定表を作る。
- 再ダウンロードの統合と別録音を誤統合しないことを確認する。
- 検証コードの再現コマンドと非機密fixtureのテスト結果を記録する。

検証コマンドの未指定部分は実装したテスト名に合わせて具体化し、実行結果・環境をこのファイルへ追記する。上記は予定であり、現在の合格結果ではない。

## Acceptance criteria

採用パーサーと購入判定・同一性の根拠が記録され、実ファイル未確認の項目を合格扱いにしない。

## Handoff notes

- 前提タスクの成果と検証記録を確認してから着手する。未解決の設計変更は該当承認ゲートへ戻す。
- 予定領域外の変更、共有Cargo設定・router・migrationへの変更が必要なら統合担当へ集約し、並行タスクの所有範囲を侵さない。
- UI変更時はdesign.mdを全文読み、依存関係・連携を変更したらMermaid図も更新する。
- 完了時にStatusをDoneへ更新し、変更ファイル、検証結果、未解決事項を記録する。購入音源・購入者情報はコミットしない。
