// Optional independent-browser fallback when the in-app browser is unavailable.
// Uses an isolated profile. Personal audio requires an explicit local file setting.
import assert from 'node:assert/strict';
import { pathToFileURL } from 'node:url';
import { resolve } from 'node:path';
import { makeServer } from './server.mjs';
const packagePath = process.env.PLAYWRIGHT_PATH || '.local/browser/node_modules/playwright/index.mjs';
const { chromium } = await import(pathToFileURL(resolve(packagePath)));
const audioFile = process.env.RUSTUNE_TEST_AUDIO;
if (audioFile && process.env.CI) throw new Error('Local audio is not allowed in CI.');
const server = makeServer(audioFile ? { audioFile } : {});
await new Promise(resolve => server.listen(0,'127.0.0.1',resolve));
let browser;
try {
  browser = await chromium.launch({ executablePath: process.env.CHROME_PATH || '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome', headless:true });
  const page = await browser.newPage();
  const origin = `http://127.0.0.1:${server.address().port}`;
  await page.route('**/*', route => new URL(route.request().url()).origin === origin ? route.continue() : route.abort());
  const errors = [], ranges = [];
  page.on('pageerror',error => errors.push(error.message));
  page.on('response',response => { if (response.url().endsWith('/audio.m4a')) ranges.push(response.status()); });
  await page.goto(origin);
  console.log(await page.locator('main').ariaSnapshot());
  await page.waitForFunction(() => document.querySelector('audio').duration > 10);
  assert.equal(await page.locator('link[rel="manifest"]').count(),0);
  await page.getByRole('button',{ name:'再生', exact:true }).click();
  await page.waitForFunction(() => !document.querySelector('audio').paused && document.querySelector('audio').currentTime > 0);
  const duration = await page.locator('audio').evaluate(audio => audio.duration);
  await page.getByRole('button',{ name:'6秒へシーク', exact:true }).click();
  await page.waitForFunction(() => document.querySelector('audio').currentTime >= 6);
  await page.waitForFunction(() => {
    const audio = document.querySelector('audio');
    return !audio.seeking && !audio.paused && audio.currentTime > 6.2 && audio.readyState >= 2 && !audio.error;
  });
  await page.getByRole('button',{ name:'一時停止', exact:true }).click();
  await page.waitForFunction(() => document.querySelector('audio').paused);
  await page.getByRole('button',{ name:'次の曲', exact:true }).click();
  await page.waitForFunction(() => document.title === '合成テスト 2 — Rustune');
  await page.getByRole('button',{ name:'前の曲', exact:true }).click();
  await page.waitForFunction(() => document.title === '合成テスト 1 — Rustune');
  const evidence = await page.evaluate(() => ({ mediaSession:'mediaSession' in navigator, metadata:navigator.mediaSession?.metadata?.title, state:navigator.mediaSession?.playbackState, events:document.getElementById('events').textContent }));
  assert.equal(evidence.mediaSession,true); assert.equal(evidence.metadata,'合成テスト 1');
  assert.ok(evidence.events.includes('registered:nexttrack'));
  assert.ok(evidence.events.includes('registered:previoustrack'));
  await page.getByRole('button',{ name:'セッション解除', exact:true }).click();
  await page.waitForFunction(() => navigator.mediaSession.metadata === null && document.querySelector('audio').paused);
  for (const width of [390,1280]) {
    await page.setViewportSize({ width,height:800 });
    assert.equal(await page.evaluate(() => document.documentElement.scrollWidth > innerWidth),false);
  }
  assert.deepEqual(errors,[]); assert.ok(ranges.includes(206));
  console.log(JSON.stringify({ chrome:await browser.version(), headless:true, syntheticAAC:!audioFile, localAudio:!!audioFile, duration, httpMediaResponses:ranges, evidence, pageErrors:errors, nativeInstallAndOSControls:'NOT TESTED' },null,2));
} finally { await browser?.close(); await new Promise(resolve => server.close(resolve)); }
