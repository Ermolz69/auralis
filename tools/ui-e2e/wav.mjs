import assert from 'node:assert/strict';
import crypto from 'node:crypto';
import fs from 'node:fs';
import fsp from 'node:fs/promises';

export async function inspectWav(file, remaining = () => Infinity) {
  const handle = await fsp.open(file, 'r');
  let data, format;
  try {
    const size = (await handle.stat()).size;
    const header = Buffer.alloc(12);
    assert.equal((await handle.read(header, 0, 12, 0)).bytesRead, 12);
    assert.equal(header.toString('ascii', 0, 4), 'RIFF');
    assert.equal(header.toString('ascii', 8, 12), 'WAVE');
    assert.equal(header.readUInt32LE(4) + 8, size);
    let offset = 12;
    for (let chunk = 0; offset + 8 <= size && chunk < 1024; chunk++) {
      const part = Buffer.alloc(8);
      await handle.read(part, 0, 8, offset);
      const bytes = part.readUInt32LE(4);
      assert.ok(offset + 8 + bytes <= size, 'WAV chunk escapes the file');
      if (part.toString('ascii', 0, 4) === 'fmt ') {
        assert.ok(bytes >= 16 && bytes <= 4096);
        const fmt = Buffer.alloc(16);
        await handle.read(fmt, 0, 16, offset + 8);
        assert.equal(fmt.readUInt16LE(0), 1);
        assert.equal(fmt.readUInt16LE(14), 16);
        format = { sample_rate: fmt.readUInt32LE(4), channels: fmt.readUInt16LE(2) };
      }
      if (part.toString('ascii', 0, 4) === 'data') {
        assert.ok(!data, 'Duplicate PCM data');
        data = { offset: offset + 8, bytes };
      }
      offset += 8 + bytes + (bytes % 2);
    }
    assert.ok(format && data && data.bytes > 0, 'Missing PCM format or data');
  } finally {
    await handle.close();
  }
  assert.equal(format.sample_rate, 48000);
  assert.ok([1, 2].includes(format.channels));
  assert.equal(data.bytes % (2 * format.channels), 0);
  const digest = crypto.createHash('sha256');
  let peak = 0,
    full = 0;
  for await (const block of fs.createReadStream(file, {
    start: data.offset,
    end: data.offset + data.bytes - 1,
    highWaterMark: 65536,
  })) {
    remaining();
    digest.update(block);
    for (let i = 0; i < block.length; i += 2) {
      const sample = block.readInt16LE(i);
      peak = Math.max(peak, Math.abs(sample));
      if (sample === 32767 || sample === -32768) full++;
    }
  }
  assert.ok(peak > 0, 'Speech WAV contains only digital silence');
  return {
    ...format,
    frames: data.bytes / (2 * format.channels),
    pcm_sha256: digest.digest('hex'),
    peak_sample_abs: peak,
    full_scale_samples: full,
  };
}
