// Explicitly launched manual verification only. Never starts during automated tests.
import { mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { resolve, join } from 'node:path';
import { pathToFileURL } from 'node:url';
import { makeServer } from './server.mjs';

if (process.env.CI) throw Error('Interactive verification is not allowed in CI.');
const packagePath =
  process.env.PLAYWRIGHT_PATH || '.local/browser/node_modules/playwright/index.mjs';
const { chromium } = await import(pathToFileURL(resolve(packagePath)));
const profile = await mkdtemp(join(tmpdir(), 'rustune-browser-check-'));
const audioFile = process.env.RUSTUNE_TEST_AUDIO;
const server = makeServer(audioFile ? { audioFile } : {});
let context;
let closing = false;
async function stop() {
  if (closing) return;
  closing = true;
  await context?.close();
  server.closeAllConnections();
  await new Promise((resolve) => server.close(resolve));
  // Only the exact directory allocated above is removed, never a user profile.
  await rm(profile, { recursive: true, force: true });
}
try {
  await new Promise((resolve, reject) => {
    server.once('error', reject);
    server.listen(Number(process.env.PORT || 4317), '127.0.0.1', resolve);
  });
  const origin = `http://127.0.0.1:${server.address().port}`;
  context = await chromium.launchPersistentContext(profile, {
    executablePath:
      process.env.CHROME_PATH || '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',
    headless: false,
    serviceWorkers: 'block',
  });
  // Local real audio is never exposed to third-party page requests.
  if (audioFile)
    await context.route('**/*', (route) =>
      new URL(route.request().url()).origin === origin ? route.continue() : route.abort(),
    );
  const page = context.pages()[0] || (await context.newPage());
  page.on('console', () => {}); // Do not collect arbitrary page messages or tags.
  await page.goto(origin);
  console.log(`Manual verification: ${origin}`);
  console.log('Temporary isolated Chrome profile. Existing personal profiles are not used.');
  console.log(
    'Use README checklist; install/Dock/media keys/Control Center results require a person.',
  );
  if (audioFile)
    console.log(
      'Local designated audio: external links blocked. Use synthetic mode for external-link checks.',
    );
  console.log(
    'Keep this process running for Dock/reopen checks. Ctrl+C closes the server/browser and removes the temporary profile.',
  );
  console.log(
    'If you install an OS app manually, remove that test app yourself before finishing; profile cleanup does not uninstall OS apps.',
  );
  await new Promise((resolve) => {
    process.once('SIGINT', resolve);
    process.once('SIGTERM', resolve);
  });
} finally {
  await stop();
}
