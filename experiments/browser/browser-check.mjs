// タスク03のブラウザ成立性検証を、独立したChrome（一時プロファイル・headless）で自動実行するスクリプト。
//
// 責務：server.mjsで検証ページと音源を配信し、Chromeで次を確認する。
//   - AAC音源のRange配信（HTTP 206）と再生・シーク
//   - Media Session（metadata、playbackState、位置情報、操作ハンドラー）の登録・解除・再登録
// 既定では合成音源（generate-fixture.mjsで生成）を使う。実音源は環境変数RUSTUNE_TEST_AUDIOで明示したときだけ使い、
// CI環境（CI変数あり）では実音源の指定を拒否する。
import assert from 'node:assert/strict';
import { pathToFileURL } from 'node:url';
import { resolve } from 'node:path';
import { mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { makeServer } from './server.mjs';
const packagePath =
  process.env.PLAYWRIGHT_PATH || '.local/browser/node_modules/playwright/index.mjs';
const { chromium } = await import(pathToFileURL(resolve(packagePath)));
const audioFile = process.env.RUSTUNE_TEST_AUDIO;
if (audioFile && process.env.CI) throw new Error('Local audio is not allowed in CI.');
const server = makeServer(audioFile ? { audioFile } : {});
await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
let browser;
let context;
const profile = await mkdtemp(join(tmpdir(), 'rustune-browser-test-'));
try {
  context = await chromium.launchPersistentContext(profile, {
    executablePath:
      process.env.CHROME_PATH || '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',
    headless: true,
  });
  browser = context.browser();
  const page = await context.newPage();
  const origin = `http://127.0.0.1:${server.address().port}`;
  await page.route('**/*', (route) =>
    new URL(route.request().url()).origin === origin ? route.continue() : route.abort(),
  );
  const errors = [],
    ranges = [];
  page.on('pageerror', (error) => errors.push(error.message));
  page.on('response', (response) => {
    if (response.url().endsWith('/audio.m4a')) ranges.push(response.status());
  });
  await page.goto(origin);
  console.log(await page.locator('main').ariaSnapshot());
  await page.waitForFunction(() => document.querySelector('audio').duration > 10);
  assert.equal(await page.locator('link[rel="manifest"]').count(), 1);
  const cdp = await page.context().newCDPSession(page);
  const manifest = await cdp.send('Page.getAppManifest');
  assert.deepEqual(manifest.errors, []);
  const manifestData = JSON.parse(manifest.data);
  assert.equal(manifestData.name, 'Rustune');
  assert.equal(manifestData.display, 'standalone');
  const installability = await cdp.send('Page.getInstallabilityErrors');
  assert.deepEqual(installability.installabilityErrors, []);
  assert.equal(
    await page.evaluate(async () => (await navigator.serviceWorker.getRegistrations()).length),
    0,
  );
  await page.getByRole('button', { name: '再生', exact: true }).click();
  await page.waitForFunction(
    () =>
      !document.querySelector('audio').paused && document.querySelector('audio').currentTime > 0,
  );
  const duration = await page.locator('audio').evaluate((audio) => audio.duration);
  await page.getByRole('button', { name: '6秒へシーク', exact: true }).click();
  await page.waitForFunction(() => document.querySelector('audio').currentTime >= 6);
  await page.waitForFunction(() => {
    const audio = document.querySelector('audio');
    return (
      !audio.seeking &&
      !audio.paused &&
      audio.currentTime > 6.2 &&
      audio.readyState >= 2 &&
      !audio.error
    );
  });
  await page.getByRole('button', { name: '一時停止', exact: true }).click();
  await page.waitForFunction(() => document.querySelector('audio').paused);
  await page.getByRole('button', { name: '次の曲', exact: true }).click();
  await page.waitForFunction(() => document.title === '合成テスト 2 — Rustune');
  await page.getByRole('button', { name: '前の曲', exact: true }).click();
  await page.waitForFunction(() => document.title === '合成テスト 1 — Rustune');
  const evidence = await page.evaluate(() => ({
    mediaSession: 'mediaSession' in navigator,
    metadata: navigator.mediaSession?.metadata?.title,
    state: navigator.mediaSession?.playbackState,
    events: document.getElementById('events').textContent,
  }));
  assert.equal(evidence.mediaSession, true);
  assert.equal(evidence.metadata, '合成テスト 1');
  assert.ok(evidence.events.includes('registered:nexttrack'));
  assert.ok(evidence.events.includes('registered:previoustrack'));
  // 再生中に解除し、遅れて届くpauseイベントとタイマーが状態を戻さないことを確認する。
  assert.equal(await page.evaluate(() => document.querySelector('audio').paused), false);
  await page.evaluate(() => {
    const session = navigator.mediaSession;
    const original = session.setPositionState.bind(session);
    window.__positionCalls = [];
    session.setPositionState = (...args) => {
      window.__positionCalls.push(args.length === 0 ? 'clear' : 'set');
      return original(...args);
    };
    // OS操作の代わりに、登録中のハンドラーを直接呼べるよう記録する。
    const originalSetHandler = session.setActionHandler.bind(session);
    window.__handlers = {};
    session.setActionHandler = (action, handler) => {
      window.__handlers[action] = handler;
      return originalSetHandler(action, handler);
    };
  });
  const pausesBeforeClear = await page.evaluate(
    () => (document.getElementById('events').textContent.match(/audio:pause/g) || []).length,
  );
  await page.getByRole('button', { name: 'セッション解除', exact: true }).click();
  await page.waitForFunction(
    () => navigator.mediaSession.metadata === null && document.querySelector('audio').paused,
  );
  await page.waitForFunction(
    (count) =>
      (document.getElementById('events').textContent.match(/audio:pause/g) || []).length > count,
    pausesBeforeClear,
  );
  await page.waitForTimeout(1200); // 500ms間隔のタイマーを2回以上経過させる
  const cleared = await page.evaluate(() => ({
    state: navigator.mediaSession.playbackState,
    positionCalls: window.__positionCalls,
  }));
  assert.equal(cleared.state, 'none');
  // 記録開始から解除のクリックまでにも500msタイマーは動き得るので、解除前の'set'は許容する。
  // 確認するのは、解除（'clear'）が呼ばれ、それ以降に位置情報が再発行されないこと。
  const firstClear = cleared.positionCalls.indexOf('clear');
  assert.ok(firstClear >= 0, `setPositionState()が呼ばれていない: ${cleared.positionCalls}`);
  assert.deepEqual(
    cleared.positionCalls.slice(firstClear).filter((call) => call === 'set'),
    [],
  );
  assert.equal(
    await page.evaluate(() =>
      ['play', 'pause', 'previoustrack', 'nexttrack', 'seekto'].every(
        (action) => window.__handlers[action] === null,
      ),
    ),
    true,
  );
  // 解除後に再生すると、ハンドラーが再登録されOSの前後操作がdispatchへ届くこと。
  await page.getByRole('button', { name: '再生', exact: true }).click();
  // playbackStateは非同期のplayingイベントで更新されるため、到着を待つ。
  await page.waitForFunction(
    () =>
      !document.querySelector('audio').paused &&
      navigator.mediaSession.metadata !== null &&
      navigator.mediaSession.playbackState === 'playing',
  );
  const reactivated = await page.evaluate(() => ({
    handlers: ['play', 'pause', 'previoustrack', 'nexttrack', 'seekto'].filter(
      (action) => typeof window.__handlers[action] === 'function',
    ),
    state: navigator.mediaSession.playbackState,
  }));
  assert.deepEqual(reactivated.handlers, ['play', 'pause', 'previoustrack', 'nexttrack', 'seekto']);
  assert.equal(reactivated.state, 'playing');
  await page.evaluate(() => window.__handlers.nexttrack({ action: 'nexttrack' }));
  await page.waitForFunction(
    () =>
      document.title === '合成テスト 2 — Rustune' &&
      navigator.mediaSession.metadata?.title === '合成テスト 2',
  );
  await page.evaluate(() => window.__handlers.previoustrack({ action: 'previoustrack' }));
  await page.waitForFunction(
    () =>
      document.title === '合成テスト 1 — Rustune' &&
      navigator.mediaSession.metadata?.title === '合成テスト 1',
  );
  await page.evaluate(() => window.__handlers.pause({ action: 'pause' }));
  await page.waitForFunction(
    () =>
      document.querySelector('audio').paused && navigator.mediaSession.playbackState === 'paused',
  );
  for (const width of [390, 1280]) {
    await page.setViewportSize({ width, height: 800 });
    assert.equal(
      await page.evaluate(() => document.documentElement.scrollWidth > innerWidth),
      false,
    );
  }
  assert.deepEqual(errors, []);
  assert.ok(ranges.includes(206));
  console.log(
    JSON.stringify(
      {
        chrome: await browser.version(),
        headless: true,
        syntheticAAC: !audioFile,
        localAudio: !!audioFile,
        duration,
        httpMediaResponses: ranges,
        manifestName: manifestData.name,
        installability,
        evidence,
        cleared,
        reactivated,
        pageErrors: errors,
        nativeInstallAndOSControls: 'NOT TESTED',
      },
      null,
      2,
    ),
  );
} finally {
  await context?.close();
  await new Promise((resolve) => server.close(resolve));
  await rm(profile, { recursive: true, force: true });
}
