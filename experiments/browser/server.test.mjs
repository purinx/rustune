import test from 'node:test';
import assert from 'node:assert/strict';
import { makeServer, rangeFor } from './server.mjs';

test('single ranges, suffixes, clipping, invalid ranges, multi-range fallback', () => {
  assert.deepEqual(rangeFor('bytes=2-4', 10), { start:2, end:4 });
  assert.deepEqual(rangeFor('bytes=8-', 10), { start:8, end:9 });
  assert.deepEqual(rangeFor('bytes=-3', 10), { start:7, end:9 });
  assert.deepEqual(rangeFor('bytes=0-99', 10), { start:0, end:9 });
  for (const header of ['bytes=10-', 'bytes=9-2', 'bytes=-0', 'bytes=-', 'wrong']) assert.equal(rangeFor(header,10),false);
  assert.equal(rangeFor('bytes=0-1,5-6',10),null);
});
test('HTTP full body, partial bytes, HEAD and 416 contracts', async () => {
  const server = makeServer();
  await new Promise(resolve => server.listen(0,'127.0.0.1',resolve));
  try {
    const url = `http://127.0.0.1:${server.address().port}/audio.m4a`;
    const full = await fetch(url), bytes = Buffer.from(await full.arrayBuffer());
    assert.equal(full.status,200); assert.equal(full.headers.get('content-type'),'audio/mp4');
    assert.equal(full.headers.get('accept-ranges'),'bytes');
    const partial = await fetch(url,{ headers:{ Range:'bytes=4-11' } });
    assert.equal(partial.status,206); assert.equal(partial.headers.get('content-range'),`bytes 4-11/${bytes.length}`);
    assert.deepEqual(Buffer.from(await partial.arrayBuffer()),bytes.subarray(4,12));
    const head = await fetch(url,{ method:'HEAD' });
    assert.equal(head.status,200); assert.equal((await head.arrayBuffer()).byteLength,0);
    assert.equal(Number(head.headers.get('content-length')),bytes.length);
    const invalid = await fetch(url,{ headers:{ Range:`bytes=${bytes.length}-` } });
    assert.equal(invalid.status,416); assert.equal(invalid.headers.get('content-range'),`bytes */${bytes.length}`);
    const multi = await fetch(url,{ headers:{ Range:'bytes=0-1,5-6' } });
    assert.equal(multi.status,200); assert.deepEqual(Buffer.from(await multi.arrayBuffer()),bytes);
  } finally { await new Promise(resolve => server.close(resolve)); }
});
