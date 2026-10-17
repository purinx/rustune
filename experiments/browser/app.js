const audio = document.querySelector('#audio');
const events = document.querySelector('#events');
let track = 1;
const log = (message) => { events.textContent = `${message}\n${events.textContent}`.slice(0, 6000); };
function metadata() {
  document.title = `合成テスト ${track} — Rustune`;
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
  if ('mediaSession' in navigator) navigator.mediaSession.playbackState = audio.paused ? 'paused' : 'playing';
  if (event === 'ended') dispatch('nexttrack');
});
if ('mediaSession' in navigator) {
  for (const action of ['play','pause','previoustrack','nexttrack','seekto']) {
    try { navigator.mediaSession.setActionHandler(action, details => dispatch(action, details)); log(`registered:${action}`); }
    catch { log(`unsupported:${action}`); }
  }
}
document.getElementById('clear').onclick = () => {
  audio.pause();
  if ('mediaSession' in navigator) {
    navigator.mediaSession.metadata = null; navigator.mediaSession.playbackState = 'none';
    for (const action of ['play','pause','previoustrack','nexttrack','seekto']) {
      try { navigator.mediaSession.setActionHandler(action, null); } catch { /* unsupported action */ }
    }
  }
  log('session:cleared');
};
setInterval(() => {
  document.getElementById('status').textContent = `曲 ${track} / ${audio.paused ? '停止' : '再生中'} / ${audio.currentTime.toFixed(1)}秒 / ${Number.isFinite(audio.duration) ? audio.duration.toFixed(1) : '?'}秒`;
  if ('mediaSession' in navigator && navigator.mediaSession.setPositionState && Number.isFinite(audio.duration) && audio.duration > 0) {
    try { navigator.mediaSession.setPositionState({ duration:audio.duration, playbackRate:audio.playbackRate, position:Math.min(audio.currentTime,audio.duration) }); } catch (error) { log(`position:${error.name}`); }
  }
}, 500);
