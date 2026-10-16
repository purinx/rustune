// Run after building the WASM assets and starting the server.
// npm install --prefix .local/browser --no-audit --no-fund playwright
// node scripts/check-foundation.mjs
import assert from 'node:assert/strict';
import { mkdir } from 'node:fs/promises';
import { chromium } from '../.local/browser/node_modules/playwright/index.mjs';

const browser = await chromium.launch({
  executablePath: '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',
  headless: true,
});
try {
  await mkdir('.local/screenshots', { recursive: true });
  for (const width of [1280, 390]) {
    const page = await browser.newPage({ viewport: { width, height: 800 } });
    const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    await page.goto('http://localhost:3867');
    await page.getByRole('heading', { name: '好きな音楽を、あなたの場所に。' }).waitFor();
    await page.getByRole('button', { name: '接続を確認', exact: true }).click();
    await page.getByRole('status').filter({ hasText: 'Rustuneサーバーに接続しました' }).waitFor();
    assert.equal(await page.evaluate(() => document.documentElement.scrollWidth > innerWidth), false);
    assert.deepEqual(errors, []);
    const icon = await page.request.get('http://localhost:3867/icon.svg');
    assert.equal(icon.status(), 200);
    assert.match(await icon.text(), /<svg/);
    await page.screenshot({ path: `.local/screenshots/foundation-${width}.png`, fullPage: true });
    console.log(`PASS: ${width}px — WASM mounted, backend connected, no horizontal overflow or page errors`);
    await page.close();
  }
} finally {
  await browser.close();
}
