import { createServer } from 'node:http';
import { readFile, stat } from 'node:fs/promises';
import { createReadStream } from 'node:fs';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('.', import.meta.url));

// Experimental single-range server only; production uses tower-http + media lease.
export function rangeFor(header, size) {
  if (!header || header.includes(',')) return null;
  const match = /^bytes=(\d*)-(\d*)$/.exec(header);
  if (!match || (!match[1] && !match[2])) return false;
  let start = match[1] ? Number(match[1]) : Math.max(0, size - Number(match[2]));
  let end = match[1] && match[2] ? Number(match[2]) : size - 1;
  if (![start, end].every(Number.isSafeInteger) || start >= size || end < start || (!match[1] && Number(match[2]) === 0)) return false;
  return { start, end: Math.min(end, size - 1) };
}

export function makeServer({ audioFile = `${root}fixtures/tone.m4a` } = {}) {
  return createServer(async (req, res) => {
    try {
      const path = new URL(req.url, 'http://localhost').pathname;
      if (!['GET', 'HEAD'].includes(req.method)) { res.writeHead(405).end(); return; }
      if (path === '/audio.m4a') {
        const file = audioFile;
        const { size } = await stat(file);
        const range = req.method === 'HEAD' ? null : rangeFor(req.headers.range, size);
        const headers = { 'Content-Type': 'audio/mp4', 'Accept-Ranges': 'bytes', 'Cache-Control': 'private, no-store' };
        if (range === false) { res.writeHead(416, { ...headers, 'Content-Range': `bytes */${size}`, 'Content-Length': 0 }).end(); return; }
        const start = range?.start ?? 0;
        const end = range?.end ?? size - 1;
        if (range) headers['Content-Range'] = `bytes ${start}-${end}/${size}`;
        res.writeHead(range ? 206 : 200, { ...headers, 'Content-Length': end - start + 1 });
        if (req.method === 'HEAD') { res.end(); return; }
        const stream = createReadStream(file, { start, end });
        res.on('close', () => stream.destroy());
        stream.on('error', () => res.destroy());
        stream.pipe(res);
        return;
      }
      const files = { '/': ['index.html', 'text/html; charset=utf-8'], '/app.js': ['app.js', 'text/javascript'], '/style.css': ['style.css', 'text/css'], '/icon.svg': ['icon.svg', 'image/svg+xml'], '/manifest.webmanifest': ['manifest.webmanifest', 'application/manifest+json'], '/icon-192.png': ['icon-192.png', 'image/png'], '/icon-512.png': ['icon-512.png', 'image/png'] };
      if (!files[path]) { res.writeHead(404).end(); return; }
      const [file, type] = files[path];
      const body = await readFile(root + file);
      res.writeHead(200, { 'Content-Type': type, 'Content-Length': body.length });
      res.end(req.method === 'HEAD' ? undefined : body);
    } catch { res.writeHead(500).end('Audio fixture unavailable.'); }
  });
}
if (process.argv[1] === fileURLToPath(import.meta.url)) {
  makeServer().listen(Number(process.env.PORT || 4317), '127.0.0.1', () => console.log('Rustune browser experiment: http://127.0.0.1:4317'));
}
