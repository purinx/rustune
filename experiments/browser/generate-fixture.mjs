import { mkdir, writeFile, readFile, unlink } from 'node:fs/promises';
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const dir = fileURLToPath(new URL('fixtures/', import.meta.url));
await mkdir(dir, { recursive: true });
const rate = 44100, seconds = 12, length = rate * seconds * 2;
const wav = Buffer.alloc(44 + length);
wav.write('RIFF'); wav.writeUInt32LE(36 + length, 4); wav.write('WAVEfmt ', 8);
wav.writeUInt32LE(16, 16); wav.writeUInt16LE(1, 20); wav.writeUInt16LE(1, 22);
wav.writeUInt32LE(rate, 24); wav.writeUInt32LE(rate * 2, 28); wav.writeUInt16LE(2, 32); wav.writeUInt16LE(16, 34);
wav.write('data', 36); wav.writeUInt32LE(length, 40);
for (let i = 0; i < rate * seconds; i++) wav.writeInt16LE(Math.round(1500 * Math.sin(2 * Math.PI * 440 * i / rate)), 44 + i * 2);
await writeFile(dir + 'tone.wav', wav);
try { execFileSync('afconvert', ['-f', 'm4af', '-d', 'aac', '-b', '128000', dir + 'tone.wav', dir + 'tone.m4a']); }
finally { await unlink(dir + 'tone.wav'); }
// Check box ordering, rather than assuming the encoder wrote a tail moov.
let bytes = await readFile(dir + 'tone.m4a');
const boxes = [];
const spans = [];
for (let offset = 0; offset < bytes.length;) {
  const size = bytes.readUInt32BE(offset), kind = bytes.toString('ascii', offset + 4, offset + 8);
  if (size < 8 || offset + size > bytes.length) throw Error('Unexpected generated MP4 box');
  boxes.push(kind); spans.push({ kind, offset, size }); offset += size;
}
if (boxes.indexOf('moov') < boxes.indexOf('mdat')) {
  const moov = spans.find(box => box.kind === 'moov');
  const moved = Buffer.from(bytes.subarray(moov.offset, moov.offset + moov.size));
  function adjust(start, end) {
    for (let offset = start; offset < end;) {
      const size = moved.readUInt32BE(offset), kind = moved.toString('ascii', offset + 4, offset + 8);
      if (size < 8 || offset + size > end) throw Error('Unexpected nested fixture box');
      if (['moov','trak','mdia','minf','stbl'].includes(kind)) adjust(offset + 8, offset + size);
      if (kind === 'stco' || kind === 'co64') {
        const count = moved.readUInt32BE(offset + 12), width = kind === 'stco' ? 4 : 8;
        if (offset + 16 + count * width > offset + size) throw Error('Invalid fixture offsets');
        for (let i = 0; i < count; i++) {
          const at = offset + 16 + i * width;
          const old = width === 4 ? BigInt(moved.readUInt32BE(at)) : moved.readBigUInt64BE(at);
          if (old < BigInt(moov.offset + moov.size)) throw Error('Unexpected data before moov');
          const value = old - BigInt(moov.size);
          if (width === 4) moved.writeUInt32BE(Number(value),at); else moved.writeBigUInt64BE(value,at);
        }
      }
      offset += size;
    }
  }
  adjust(0,moved.length);
  bytes = Buffer.concat([bytes.subarray(0,moov.offset),bytes.subarray(moov.offset + moov.size),moved]);
  await writeFile(dir + 'tone.m4a',bytes);
  boxes.splice(boxes.indexOf('moov'),1); boxes.push('moov');
}
console.log(JSON.stringify({ synthetic: true, seconds, bytes: bytes.length, boxes }));
