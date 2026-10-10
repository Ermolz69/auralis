import crypto from 'node:crypto';
import fsp from 'node:fs/promises';
import path from 'node:path';

export async function verifyHistoryExports(
  dataRoot,
  publications,
  outputs,
  reopened,
  format = 'srt',
) {
  if (format !== 'srt' && format !== 'vtt')
    throw new Error(`Unsupported native historical export format: ${format}`);
  const fixtureRoot = path.resolve(dataRoot, '..', 'fixture');
  const managedRoot = path.resolve(dataRoot, 'projects');
  const names = ['source.mp4', `source.${format}`];
  for (const index of [1, 2]) {
    const publication = publications[index];
    const managedPath = path.resolve(managedRoot, ...publication.location_value.split('/'));
    if (!managedPath.startsWith(`${managedRoot}${path.sep}`))
      throw new Error('Historical export verification escaped managed storage');
    const expected = await fsp.readFile(managedPath);
    const exportName = `native-export-${publication.result_id}.${format}`;
    names.push(exportName);
    await verifyExactFile(path.join(fixtureRoot, exportName), expected, outputs[index].sha256);
    if (reopened && index === 1) {
      const reexportName = `native-reexport-${publication.result_id}.${format}`;
      names.push(reexportName);
      await verifyExactFile(path.join(fixtureRoot, reexportName), expected, outputs[index].sha256);
    }
  }
  const found = (await fsp.readdir(fixtureRoot)).sort();
  if (JSON.stringify(found) !== JSON.stringify(names.sort()))
    throw new Error(`Historical export left unexpected or missing fixture files: ${found}`);
}

async function verifyExactFile(filename, expected, digest) {
  const actual = await fsp.readFile(filename);
  const actualDigest = crypto.createHash('sha256').update(actual).digest('hex');
  if (!actual.equals(expected) || actualDigest !== digest)
    throw new Error(`Native historical export differs from its managed result: ${filename}`);
}
