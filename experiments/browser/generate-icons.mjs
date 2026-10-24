// Render the existing original Rustune SVG to install-sized PNGs locally.
import { readFile } from 'node:fs/promises';
import { pathToFileURL, fileURLToPath } from 'node:url';
import { resolve } from 'node:path';
const { chromium } = await import(
  pathToFileURL(
    resolve(process.env.PLAYWRIGHT_PATH || '.local/browser/node_modules/playwright/index.mjs'),
  )
);
const browser = await chromium.launch({
  executablePath:
    process.env.CHROME_PATH || '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',
  headless: true,
});
try {
  const svg = await readFile(new URL('icon.svg', import.meta.url), 'utf8');
  const page = await browser.newPage();
  await page.route('**/*', (route) => route.abort());
  for (const size of [192, 512]) {
    await page.setViewportSize({ width: size, height: size });
    await page.setContent(
      `<style>body{margin:0}svg{display:block;width:${size}px;height:${size}px}</style>${svg}`,
    );
    await page.locator('svg').screenshot({
      path: fileURLToPath(new URL(`icon-${size}.png`, import.meta.url)),
      omitBackground: true,
    });
  }
} finally {
  await browser.close();
}
