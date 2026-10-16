# 12: WASMプレイヤーとOSメディア操作

Status: Planned
Project: 20261012-214613-personal-music-library
Requirements: REQ-007, REQ-008, REQ-016, NFR-002, NFR-004
Depends on: 08, 11
Parallel group: None

## Objective

画面移動で途切れない単一プレイヤーをUI・OSの両方から操作できるようにする。

## Design references

MODULE_DESIGN §15、03の検証記録。プロジェクトの承認済みPRD・DESIGN_DOC・MODULE_DESIGNを前提とする。

## Scope

### Included

以下の実装手順と検証を含む。

### Excluded

複数ウィンドウ同期、オフライン、ギャップレス。

## Expected code areas

crates/web/src/player/、media_session/、再生バー部品・ブラウザテスト

## Implementation steps

1. AppShell生存期間のaudio要素・controller・Closure所有と解放を実装する。
2. domainキューを接続し、play/pause/seek/volume/next/previous/endedを統一dispatchへ集約する。
3. generation、currentSrc、Promise拒否、load中pause、削除曲処理を実装する。
4. Media Sessionのmetadata、handler、再生状態を接続し機能検出を行う。

## Verification

- 実配信を使い、画面遷移・素早い曲変更・末尾・削除・再生拒否を確認する。
- Mac Chromeで背景操作・メディアキー・コントロールセンターを確認する。

検証コマンドの未指定部分は実装したテスト名に合わせて具体化し、実行結果・環境をこのファイルへ追記する。上記は予定であり、現在の合格結果ではない。

## Acceptance criteria

OSとUIで同じキューと実audio状態を操作し、曲変更の古いイベントが現在曲を壊さない。

## Handoff notes

- 前提タスクの成果と検証記録を確認してから着手する。未解決の設計変更は該当承認ゲートへ戻す。
- 予定領域外の変更、共有Cargo設定・router・migrationへの変更が必要なら統合担当へ集約し、並行タスクの所有範囲を侵さない。
- UI変更時はdesign.mdを全文読み、依存関係・連携を変更したらMermaid図も更新する。
- 完了時にStatusをDoneへ更新し、変更ファイル、検証結果、未解決事項を記録する。購入音源・購入者情報はコミットしない。
