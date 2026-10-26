import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { cp, mkdir, mkdtemp, readFile, readdir } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../../', import.meta.url));
const provenance = JSON.parse(await readFile(new URL('./provenance.json', import.meta.url)));
const packageName = `${provenance.package}-${provenance.version}`;
const vendor = path.join(root, 'vendor', packageName);
const patchedPaths = ['Cargo.toml', 'src/tokio/job_object.rs', 'src/windows.rs'];
const digest = (bytes) => createHash('sha256').update(bytes).digest('hex');

async function files(directory, prefix = '') {
  const result = [];
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    const name = prefix + entry.name;
    if (entry.isDirectory())
      result.push(...(await files(path.join(directory, entry.name), name + '/')));
    else {
      assert.ok(entry.isFile(), `Unsupported package entry: ${name}`);
      result.push(name);
    }
  }
  return result.sort();
}

const mode = process.argv[2];
if (mode === 'patch-hashes') {
  console.log(
    JSON.stringify(
      Object.fromEntries(
        await Promise.all(
          patchedPaths.map(async (name) => [name, digest(await readFile(path.join(vendor, name)))]),
        ),
      ),
      null,
      2,
    ),
  );
} else {
  assert.ok(['import', 'verify'].includes(mode));
  const archive = path.resolve(process.argv[3]);
  assert.equal(
    digest(await readFile(archive)),
    provenance.sha256,
    'Upstream archive checksum mismatch',
  );
  const entries = execFileSync('tar', ['-tzf', archive], { encoding: 'utf8' })
    .trim()
    .split(/\r?\n/);
  assert.ok(
    entries.every((name) => name.startsWith(packageName + '/') && !name.split('/').includes('..')),
  );
  const extraction = await mkdtemp(path.join(os.tmpdir(), 'auralis-process-wrap-source-'));
  execFileSync('tar', ['-xzf', archive, '-C', extraction]);
  const original = path.join(extraction, packageName);
  const originalFiles = await files(original);
  const vcs = JSON.parse(await readFile(path.join(original, '.cargo_vcs_info.json')));
  assert.equal(vcs.git.sha1, provenance.upstreamRevision);
  if (mode === 'import') {
    await mkdir(vendor);
    for (const entry of await readdir(original)) {
      await cp(path.join(original, entry), path.join(vendor, entry), {
        recursive: true,
        force: false,
        errorOnExist: true,
      });
    }
    for (const name of originalFiles) {
      assert.equal(
        digest(await readFile(path.join(vendor, name))),
        digest(await readFile(path.join(original, name))),
      );
    }
    console.log(`Imported and verified ${originalFiles.length} pristine ${packageName} files.`);
  } else {
    assert.deepEqual(await files(vendor), originalFiles, 'Vendored package file set changed');
    assert.deepEqual(Object.keys(provenance.patchedFiles).sort(), patchedPaths.slice().sort());
    for (const name of originalFiles) {
      const expected =
        provenance.patchedFiles[name] ?? digest(await readFile(path.join(original, name)));
      assert.equal(
        digest(await readFile(path.join(vendor, name))),
        expected,
        `Unexpected vendor edit: ${name}`,
      );
    }
    const manifest = await readFile(path.join(root, 'Cargo.toml'), 'utf8');
    assert.match(manifest, /process-wrap = \{ path = "vendor\/process-wrap-10\.0\.0" \}/);
    const lock = (await readFile(path.join(root, 'Cargo.lock'), 'utf8'))
      .split('[[package]]')
      .filter((entry) => /^name = "process-wrap"$/m.test(entry));
    assert.equal(lock.length, 1);
    assert.match(lock[0], /^version = "10\.0\.0"$/m);
    assert.doesNotMatch(lock[0], /^(source|checksum) = /m);
    console.log(
      `Verified ${originalFiles.length} ${packageName} files; exactly three pinned local edits.`,
    );
  }
}
