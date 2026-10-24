// タスク03のブラウザ成立性検証ページの動作。製品コードではなく、検証用の実験コード。
//
// 責務：<audio>で音源を再生し、Media Session API（OSのメディアキー・コントロールセンター連携）を操作する。
//   - metadata()：曲情報をOSへ渡し、未登録なら操作ハンドラーを登録する
//   - dispatch()：OSやボタンからの操作（再生・一時停止・前後の曲・シーク）を<audio>へ反映する
//   - セッション解除：metadata・ハンドラー・位置情報を消し、以後のイベントやタイマーが状態を書き戻さないようにする
// 実行結果は#eventsへログとして出し、browser-check.mjsがそのログと状態を検証する。
const audio = document.querySelector('#audio');
const events = document.querySelector('#events');
let track = 1;
const ACTIONS = ['play','pause','previoustrack','nexttrack','seekto'];
let handlersRegistered = false;
const log = (message) => { events.textContent = `${message}\n${events.textContent}`.slice(0, 6000); };
// セッション解除でハンドラーを外すため、再生を再開するたびに未登録なら登録し直す。
function registerHandlers() {
  if (handlersRegistered || !('mediaSession' in navigator)) return;
  for (const action of ACTIONS) {
    try { navigator.mediaSession.setActionHandler(action, details => dispatch(action, details)); log(`registered:${action}`); }
    catch { log(`unsupported:${action}`); }
  }
  handlersRegistered = true;
}
function metadata() {
  document.title = `合成テスト ${track} — Rustune`;
  registerHandlers();
  if ('mediaSession' in navigator) navigator.mediaSession.metadata = new MediaMetadata({ title: `合成テスト ${track}`, artist: 'Rustune', album: 'ブラウザ成立性検証', artwork: [{ src: '/icon.svg', sizes: '192x192', type: 'image/svg+xml' }] });
}
async function dispatch(action, details = {}) {
  log(`command:${action}`);
  try {
    if (action === 'play') { metadata(); await audio.play(); }
    if (action === 'pause') audio.pause();
    if (action === 'nexttrack' || action === 'previoustrack') {
      track = track === 1 ? 2 : 1; audio.currentTime = 0; metadata(); await audio.play();
    }
    if (action === 'seekto') audio.currentTime = Math.min(audio.duration || 0, Math.max(0, details.seekTime ?? 6));
  } catch (error) { log(`error:${error.name}`); }
}
const mapping = { play:'play', pause:'pause', previous:'previoustrack', next:'nexttrack', seek:'seekto' };
for (const [id, action] of Object.entries(mapping)) document.getElementById(id).onclick = () => dispatch(action);
for (const event of ['loadedmetadata', 'playing', 'pause', 'seeked', 'ended', 'error']) audio.addEventListener(event, () => {
  log(`audio:${event}`);
  // セッション解除後に遅れて届くpause等で、playbackStateを'none'から戻さない。
  if ('mediaSession' in navigator && navigator.mediaSession.metadata) navigator.mediaSession.playbackState = audio.paused ? 'paused' : 'playing';
  if (event === 'ended') dispatch('nexttrack');
});
registerHandlers();
document.getElementById('clear').onclick = () => {
  audio.pause();
  if ('mediaSession' in navigator) {
    navigator.mediaSession.metadata = null; navigator.mediaSession.playbackState = 'none';
    // 引数なしの呼び出しで位置情報を明示的に消去する。
    try { navigator.mediaSession.setPositionState?.(); } catch (error) { log(`position:${error.name}`); }
    for (const action of ACTIONS) {
      try { navigator.mediaSession.setActionHandler(action, null); } catch { /* unsupported action */ }
    }
    handlersRegistered = false;
  }
  log('session:cleared');
};
setInterval(() => {
  document.getElementById('status').textContent = `曲 ${track} / ${audio.paused ? '停止' : '再生中'} / ${audio.currentTime.toFixed(1)}秒 / ${Number.isFinite(audio.duration) ? audio.duration.toFixed(1) : '?'}秒`;
  // セッション解除後（metadataがnull）は位置情報を再発行しない。
  if ('mediaSession' in navigator && navigator.mediaSession.metadata && navigator.mediaSession.setPositionState && Number.isFinite(audio.duration) && audio.duration > 0) {
    try { navigator.mediaSession.setPositionState({ duration:audio.duration, playbackRate:audio.playbackRate, position:Math.min(audio.currentTime,audio.duration) }); } catch (error) { log(`position:${error.name}`); }
  }
}, 500);
