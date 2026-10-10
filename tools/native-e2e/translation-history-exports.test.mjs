import assert from 'node:assert/strict';
import crypto from 'node:crypto';
import fsp from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';
import { verifyHistoryExports } from './translation-history-exports.mjs';

for (const format of ['srt', 'vtt'])
  test(`native ${format} export verifier rejects changed bytes and staging files`, async () => {
    const tempBase = path.resolve(os.tmpdir());
    const root = await fsp.mkdtemp(path.join(tempBase, 'auralis-native-history-export-test-'));
    const dataRoot = path.join(root, 'app-data');
    const fixtureRoot = path.join(root, 'fixture');
    const managedRoot = path.join(dataRoot, 'projects', 'translation');
    try {
      await fsp.mkdir(managedRoot, { recursive: true });
      await fsp.mkdir(fixtureRoot, { recursive: true });
      await fsp.writeFile(path.join(fixtureRoot, 'source.mp4'), Buffer.alloc(0));
      await fsp.writeFile(path.join(fixtureRoot, `source.${format}`), Buffer.alloc(0));
      const bytes = [Buffer.from('base'), Buffer.from('later'), Buffer.from('branch')];
      const publications = [];
      const outputs = [];
      for (const [index, content] of bytes.entries()) {
        const resultId = `result-${index}`;
        const location = `translation/${resultId}.${format}`;
        publications.push({ result_id: resultId, location_value: location });
        outputs.push({ sha256: crypto.createHash('sha256').update(content).digest('hex') });
        await fsp.writeFile(path.join(managedRoot, `${resultId}.${format}`), content);
        if (index > 0)
          await fsp.writeFile(
            path.join(fixtureRoot, `native-export-${resultId}.${format}`),
            content,
          );
      }
      await verifyHistoryExports(dataRoot, publications, outputs, false, format);
      const reexport = path.join(fixtureRoot, `native-reexport-result-1.${format}`);
      await fsp.writeFile(reexport, bytes[1]);
      await verifyHistoryExports(dataRoot, publications, outputs, true, format);
      await fsp.writeFile(reexport, Buffer.from('wrong'));
      await assert.rejects(verifyHistoryExports(dataRoot, publications, outputs, true, format));
      await fsp.writeFile(reexport, bytes[1]);
      await fsp.writeFile(path.join(fixtureRoot, 'leftover.tmp'), Buffer.alloc(0));
      await assert.rejects(verifyHistoryExports(dataRoot, publications, outputs, true, format));
    } finally {
      if (!path.resolve(root).startsWith(`${tempBase}${path.sep}`))
        throw new Error('Temporary verification root escaped the system temp directory');
      await fsp.rm(root, { recursive: true, force: true });
    }
  });
