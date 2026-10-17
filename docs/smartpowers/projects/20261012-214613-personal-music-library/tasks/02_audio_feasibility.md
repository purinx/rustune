# 02: 購入音源の識別・重複防止の検証

Status: Done
Project: 20261012-214613-personal-music-library
Requirements: REQ-004, REQ-012, NFR-010
Depends on: 01
Parallel group: A

## Objective

指定された購入AACの解析・購入由来情報の取得と、同じ音源の重複判定を実証する。

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

1. 使用可能な指定実音源を確認する。生音源や購入者タグをGit・通常ログへ出さない。
2. Rustの形式検証ライブラリを比較し、AAC構造、サンプル範囲、購入由来識別の取得を試す。
3. 同じファイル・タグ違い・別パスのdigestとストアIDを比較し、採用規則と制限を記録する。
4. 成立しないMust要件があれば後続06を開始せず設計レビューへ戻す。

## Verification

- 購入AAC、CD AAC、ALAC、途中・破損ファイルの判定表を作る。
- 同じ音源の別パスコピーを統合し、異なる音声サンプルをタイトル一致だけで誤統合しないことを確認する。
- 検証コードの再現コマンドと非機密fixtureのテスト結果を記録する。

検証コマンドの未指定部分は実装したテスト名に合わせて具体化し、実行結果・環境をこのファイルへ追記する。上記は予定であり、現在の合格結果ではない。

## Acceptance criteria

採用パーサーと購入判定・同一性の根拠が記録され、実ファイル未確認の項目を合格扱いにしない。

## Handoff notes

### 2026-10-18 完了・統合検証

- 専用worktreeのsubagent成果`ebdad7c`をレビューしmainへ統合。採用候補はmp4-track 0.1.0、mp4ameta 0.13.0、top-level境界とmdat範囲を扱うmp4-atom 0.15.0の薄いアダプター。規則・判定表・制限は`experiments/audio/README.md`に記録した。
- `itunes-tag-pattern-v1`でAACと購入由来タグの組合せを確認。タグは編集可能で、権利認証ではない。購入者の値は公開結果に含めない。
- 合成12テスト成功（ローカル専用1件は通常実行で除外）。指定実音源の別パス一時コピー・メモリ内タグ編集テスト1件も成功。元ファイルは変更せず、一時コピーは自動削除。fmt・Clippy成功。統合担当もmain上で同じ検証を実施した。
- 同じ音声のdigest不変、異なるpacketのdigest相違、mdat外sample参照の拒否を確認。全購入曲の網羅性や取得し直した音源の同一性は保証しない。
- 06へ：上限付きRead+Seekへの製品化、全階層box数予算、設計の200 MiBと実験64 MiBの区別を実装・検証する。04/09へ：同じstore IDでもdigestが違う場合の識別衝突を明示し、異音声を黙って統合しない。

### 2026-10-16 実装記録

- `experiments/audio/`に独立Cargo workspace、読み取り専用の診断CLI、メモリ内の合成fixtureテストを追加した。root Cargo設定や製品コードには依存を追加していない。
- 候補比較・判定表・上限・不足点は`experiments/audio/README.md`へ記録。mp4-track 0.1.0を実験採用し、mp4ameta 0.13.0はタグ調査専用。製品パーサーの最終選定ではない。
- 購入者値・元パス・生パーサーエラーは出力せず、購入/DRM保証フラグは常にfalse。合成AACは構造検証用で、デコード試験ではない。
- 実音源の更新：ユーザー指定の1件を読み取り、AAC解析・購入日時/アカウントタグの存在確認・digest計算に成功（警告0）。この端末内の検証利用指示を受領。音源ルートと再生テスト用ファイルはGit管理対象外の`.local/audio-testing.md`に記録。実音源や購入者情報はコミット・外部送信・CI利用しない。
- 2026-10-17のユーザー指示により、保護情報の専用検査と配布元からの再取得比較を完了条件から削除した。試料の追加提供は必須とせず、StatusをIn Progressへ戻す。過去の診断フラグは保証を表さない。
- 残作業：購入由来情報の判定規則、同じファイル・タグ差分の重複判定、製品に適用する解析上限とmdat内sample参照検証の方針を記録する。未実施を合格扱いにしない。
- 検証結果：`cargo test --manifest-path experiments/audio/Cargo.toml --locked`は6テスト成功。`cargo fmt --manifest-path experiments/audio/Cargo.toml -- --check`および`cargo clippy --manifest-path experiments/audio/Cargo.toml --all-targets --locked -- -D warnings`成功。macOS / Rust 1.99.1。実購入ファイルの検証成功を意味しない。
- ファイル診断コマンド：`cargo run --manifest-path experiments/audio/Cargo.toml --locked -- '/explicit/authorized/file.m4a'`。明示的に使用指示された試料だけを指定する。

- 前提タスクの成果と検証記録を確認してから着手する。未解決の設計変更は該当承認ゲートへ戻す。
- 予定領域外の変更、共有Cargo設定・router・migrationへの変更が必要なら統合担当へ集約し、並行タスクの所有範囲を侵さない。
- UI変更時はdesign.mdを全文読み、依存関係・連携を変更したらMermaid図も更新する。
- 完了時にStatusをDoneへ更新し、変更ファイル、検証結果、未解決事項を記録する。購入音源・購入者情報はコミットしない。
