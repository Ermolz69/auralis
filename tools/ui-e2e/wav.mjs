import assert from 'node:assert/strict';
import crypto from 'node:crypto';
import fs from 'node:fs';
import fsp from 'node:fs/promises';

export async function inspectWav(file, remaining = () => Infinity, options = {}) {
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
  if (options.sample_rate !== null) assert.equal(format.sample_rate, options.sample_rate ?? 48000);
  else
    assert.ok(
      [
        8000, 11025, 12000, 16000, 22050, 24000, 32000, 44100, 48000, 88200, 96000, 176400, 192000,
      ].includes(format.sample_rate),
    );
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
  if (!options.allow_silence) assert.ok(peak > 0, 'Speech WAV contains only digital silence');
  return {
    ...format,
    frames: data.bytes / (2 * format.channels),
    pcm_sha256: digest.digest('hex'),
    peak_sample_abs: peak,
    full_scale_samples: full,
    ...(options.include_data ? { data_offset: data.offset } : {}),
  };
}

export async function wavRangeHash(file, properties, range, remaining = () => Infinity) {
  const align = properties.channels * 2,
    bytes = range.frame_count * align;
  assert.ok(Number.isSafeInteger(bytes) && bytes > 0 && bytes <= 2 ** 32 - 128);
  assert.ok(range.first_frame >= 0 && range.first_frame + range.frame_count <= properties.frames);
  const header = Buffer.alloc(44);
  header.write('RIFF');
  header.writeUInt32LE(bytes + 36, 4);
  header.write('WAVEfmt ', 8);
  header.writeUInt32LE(16, 16);
  header.writeUInt16LE(1, 20);
  header.writeUInt16LE(properties.channels, 22);
  header.writeUInt32LE(48000, 24);
  header.writeUInt32LE(48000 * align, 28);
  header.writeUInt16LE(align, 32);
  header.writeUInt16LE(16, 34);
  header.write('data', 36);
  header.writeUInt32LE(bytes, 40);
  const digest = crypto.createHash('sha256').update(header);
  const start = properties.data_offset + range.first_frame * align;
  let read = 0;
  for await (const block of fs.createReadStream(file, {
    start,
    end: start + bytes - 1,
    highWaterMark: 65536,
  })) {
    remaining();
    read += block.length;
    digest.update(block);
  }
  assert.equal(read, bytes, 'Raw utterance PCM range was truncated');
  return digest.digest('hex');
}
