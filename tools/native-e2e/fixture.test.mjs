import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';
import { startFixtureServer } from './fixture.mjs';

test('fixture server retains full, HEAD, range and invalid-range responses', async () => {
  const root = await fs.mkdtemp(path.join(os.tmpdir(), 'auralis-native-fixture-'));
  const sourcePath = path.join(root, 'source.mp4');
  let server;
  try {
    await fs.writeFile(sourcePath, '0123456789');
    server = await startFixtureServer(sourcePath);

    const full = await fetch(server.url);
    assert.equal(full.status, 200);
    assert.equal(full.headers.get('accept-ranges'), 'bytes');
    assert.equal(full.headers.get('content-type'), 'video/mp4');
    assert.equal(full.headers.get('content-length'), '10');
    assert.equal(await full.text(), '0123456789');

    const head = await fetch(server.url, { method: 'HEAD' });
    assert.equal(head.status, 200);
    assert.equal(head.headers.get('content-length'), '10');
    assert.equal(await head.text(), '');

    const range = await fetch(server.url, { headers: { range: 'bytes=2-5' } });
    assert.equal(range.status, 206);
    assert.equal(range.headers.get('content-range'), 'bytes 2-5/10');
    assert.equal(await range.text(), '2345');

    const invalid = await fetch(server.url, { headers: { range: 'bytes=10-' } });
    assert.equal(invalid.status, 416);
    assert.equal(invalid.headers.get('content-range'), 'bytes */10');
    assert.equal(server.requestCount, 4);

    const missing = await fetch(new URL('/missing', server.url));
    assert.equal(missing.status, 404);
    assert.equal(server.requestCount, 4);
  } finally {
    await server?.close();
    await fs.rm(root, { recursive: true, force: true });
  }
});
