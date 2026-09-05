import crypto from 'node:crypto';
import fs from 'node:fs';
import fsp from 'node:fs/promises';
import path from 'node:path';

export async function sha256File(filePath) {
  const hash = crypto.createHash('sha256');
  await new Promise((resolve, reject) => {
    const stream = fs.createReadStream(filePath);
    stream.on('data', (chunk) => hash.update(chunk));
    stream.on('error', reject);
    stream.on('end', resolve);
  });
  return hash.digest('hex');
}

export async function fileMatches(filePath, expectedSha256) {
  try {
    return (await sha256File(filePath)) === expectedSha256;
  } catch (error) {
    if (error?.code === 'ENOENT') return false;
    throw error;
  }
}

export async function downloadVerified({ url, sha256, destination, executable = false }) {
  if (await fileMatches(destination, sha256)) return 'cached';

  const response = await fetch(url, {
    headers: { Accept: 'application/octet-stream', 'User-Agent': 'Auralis-media-tools' },
    redirect: 'follow',
    signal: AbortSignal.timeout(180_000),
  });
  if (!response.ok) throw new Error(`Download failed with HTTP ${response.status}: ${url}`);

  const bytes = Buffer.from(await response.arrayBuffer());
  const actualSha256 = crypto.createHash('sha256').update(bytes).digest('hex');
  if (actualSha256 !== sha256) {
    throw new Error(`SHA-256 mismatch for ${url}: expected ${sha256}, received ${actualSha256}`);
  }

  await fsp.mkdir(path.dirname(destination), { recursive: true });
  const temporary = `${destination}.tmp-${process.pid}`;
  try {
    await fsp.writeFile(temporary, bytes, { mode: executable ? 0o755 : 0o644 });
    await fsp.rm(destination, { force: true });
    await fsp.rename(temporary, destination);
    if (executable && process.platform !== 'win32') await fsp.chmod(destination, 0o755);
  } finally {
    await fsp.rm(temporary, { force: true });
  }
  return 'downloaded';
}

export async function writeIfChanged(filePath, content) {
  try {
    if ((await fsp.readFile(filePath, 'utf8')) === content) return;
  } catch (error) {
    if (error?.code !== 'ENOENT') throw error;
  }
  await fsp.mkdir(path.dirname(filePath), { recursive: true });
  await fsp.writeFile(filePath, content, 'utf8');
}
