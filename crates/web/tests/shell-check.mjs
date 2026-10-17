// Build with Trunk first. This isolated shell test stubs only the foundation health API.
// PLAYWRIGHT_PATH may point to an already installed playwright/index.mjs.
import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { readFile, mkdir } from 'node:fs/promises';
import { fileURLToPath, pathToFileURL } from 'node:url';
import path from 'node:path';

const { chromium } = await import(process.env.PLAYWRIGHT_PATH
  ? pathToFileURL(path.resolve(process.env.PLAYWRIGHT_PATH)).href : 'playwright');
const root = fileURLToPath(new URL('../dist/', import.meta.url));
const shots = fileURLToPath(new URL('../../../.local/screenshots/', import.meta.url));
await mkdir(shots, { recursive: true });
let healthy = true;
const mime = { '.wasm': 'application/wasm', '.js': 'text/javascript', '.css': 'text/css',
  '.html': 'text/html', '.svg': 'image/svg+xml', '.png': 'image/png', '.webmanifest': 'application/manifest+json' };
const server = createServer(async (req, res) => {
  if (req.url === '/api/v1/health') {
    res.writeHead(healthy ? 200 : 503, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify({ service: 'rustune', version: '0.1.0', stage: 'foundation' }));
    return;
  }
  const pathname = new URL(req.url, 'http://localhost').pathname;
  const name = path.resolve(root, `.${pathname}`);
  if (name !== path.resolve(root) && !name.startsWith(root)) { res.writeHead(400); res.end(); return; }
  try {
    const contents = await readFile(name);
    res.writeHead(200, { 'Content-Type': mime[path.extname(name)] ?? 'application/octet-stream' });
    res.end(contents);
  } catch {
    if (path.extname(name)) { res.writeHead(404); res.end(); return; }
    res.writeHead(200, { 'Content-Type': 'text/html' });
    res.end(await readFile(path.join(root, 'index.html')));
  }
});
await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
const base = `http://127.0.0.1:${server.address().port}`;
const browser = await chromium.launch({ headless: true,
  executablePath: process.env.CHROME_PATH ?? '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome' });
const routes = [ ['/library', 'ライブラリ'], ['/albums', 'アルバム'], ['/artists', 'アーティスト'],
  ['/playlists', 'プレイリスト'], ['/favorites', 'お気に入り'], ['/imports', '取り込み'],
  ['/settings', '設定'], ['/exclusions', '除外した音源'] ];
try {
  for (const width of [1280, 390]) {
    const page = await browser.newPage({ viewport: { width, height: 800 } });
    const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    await page.goto(base);
    await page.getByRole('heading', { level: 1, name: 'ライブラリ', exact: true }).waitFor();
    await page.waitForURL(`${base}/library`);
    const player = await page.getByRole('region', { name: '再生コントロール（準備中）' }).elementHandle();
    for (const [route, name] of routes) {
      const changed = new URL(page.url()).pathname !== route;
      const nav = page.getByRole('navigation').getByRole('link', { name, exact: true });
      await nav.focus();
      await nav.press('Enter');
      await page.waitForURL(`${base}${route}`);
      await page.getByRole('heading', { level: 1, name, exact: true }).waitFor();
      assert.equal(await page.title(), `${name} — Rustune`);
      assert.equal(await nav.getAttribute('aria-current'), 'page');
      if (changed) await page.waitForFunction(() => document.activeElement.tagName === 'H1');
      assert.equal(await player.evaluate(element => element.isConnected), true, 'player must survive navigation');
      const box = await player.boundingBox();
      assert(box.y >= 0 && box.y + box.height <= 800, 'player stays in viewport');
      assert.equal(await page.evaluate(() => document.documentElement.scrollWidth > innerWidth), false);
    }
    const opener = page.getByRole('button', { name: 'Rustuneについて', exact: true });
    await opener.focus();
    await opener.press('Enter');
    const dialog = page.getByRole('dialog', { name: 'Rustuneについて' });
    await dialog.waitFor();
    assert.equal(await dialog.evaluate(element => element.contains(document.activeElement)), true);
    await page.keyboard.press('Shift+Tab');
    assert.equal(await dialog.evaluate(element => element.contains(document.activeElement)), true, 'modal traps focus');
    const external = dialog.getByRole('link', { name: 'iTunesの案内をブラウザで開く ↗' });
    assert.equal(await external.getAttribute('target'), '_blank');
    assert.match(await external.getAttribute('rel'), /noopener/);
    await page.keyboard.press('Escape');
    await dialog.waitFor({ state: 'hidden' });
    assert.equal(await opener.evaluate(element => document.activeElement === element), true, 'Escape restores opener focus');
    await opener.click();
    await dialog.getByRole('button', { name: '閉じる' }).click();
    await dialog.waitFor({ state: 'hidden' });
    assert.equal(await opener.evaluate(element => document.activeElement === element), true, 'close restores opener focus');
    await page.getByRole('navigation').getByRole('link', { name: '設定', exact: true }).click();
    await page.getByRole('button', { name: '接続を確認', exact: true }).click();
    await page.getByRole('status').filter({ hasText: 'Rustuneサーバーに接続しました' }).waitFor();
    healthy = false;
    await page.getByRole('button', { name: '接続を確認', exact: true }).click();
    await page.getByRole('alert').filter({ hasText: 'サーバーの起動を確認' }).waitFor();
    healthy = true;
    await page.getByRole('button', { name: '接続を確認', exact: true }).click();
    await page.getByRole('status').filter({ hasText: 'Rustuneサーバーに接続しました' }).waitFor();
    assert.equal(await page.getByRole('alert').count(), 0);
    for (const route of ['/albums/test', '/artists/test', '/playlists/test', '/not-found']) {
      await page.goto(`${base}${route}`);
      await page.getByRole('heading', { level: 2 }).waitFor();
      await page.getByRole('main').getByRole('link', { name: 'ライブラリへ', exact: true }).click();
      await page.waitForURL(`${base}/library`);
    }
    await page.reload();
    await page.getByRole('heading', { level: 1, name: 'ライブラリ', exact: true }).waitFor();
    const manifestResponse = await page.request.get(`${base}/manifest.webmanifest`);
    assert.match(manifestResponse.headers()['content-type'], /application\/manifest\+json/);
    const manifest = await manifestResponse.json();
    assert.equal(manifest.name, 'Rustune');
    assert.equal(manifest.display, 'standalone');
    assert.equal(manifest.start_url, '/');
    for (const size of [192, 512]) {
      const icon = await page.request.get(`${base}/icon-${size}.png`);
      const bytes = await icon.body();
      assert.equal(icon.status(), 200);
      assert.equal(bytes.readUInt32BE(16), size);
      assert.equal(bytes.readUInt32BE(20), size);
    }
    assert.deepEqual(errors, []);
    await page.screenshot({ path: path.join(shots, `shell-${width}.png`), fullPage: true });
    console.log(`PASS ${width}px: navigation, titles, focus, persistent player, dialogs, health states, direct routes, Manifest/icons`);
    await page.close();
  }
  console.log(`Chrome ${browser.version()}; shell-only health fixture; no audio or product business API exercised`);
} finally {
  await browser.close();
  await new Promise(resolve => server.close(resolve));
}
