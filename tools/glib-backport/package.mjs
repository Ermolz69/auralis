import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { cp, mkdir, mkdtemp, readFile, readdir, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

export const root = fileURLToPath(new URL('../../', import.meta.url));
export const provenance = JSON.parse(await readFile(new URL('./provenance.json', import.meta.url)));
const packageName = `${provenance.package}-${provenance.version}`;
export const vendor = path.join(root, 'vendor', packageName);
const sha256 = (bytes) => createHash('sha256').update(bytes).digest('hex');

export function patchIterator(source) {
  const before = 'let p: *mut libc::c_char = std::ptr::null_mut();';
  assert.equal(source.split(before).length, 2, 'Expected exactly one upstream pointer declaration');
  assert.equal(source.split('                &p,').length, 2, 'Expected exactly one out-pointer');
  return source
    .replace(before, 'let mut p: *mut libc::c_char = std::ptr::null_mut();')
    .replace('                &p,', '                &mut p,');
}

export function patchManifest(source) {
  assert.ok(!source.includes('[lints.'), 'Upstream lint policy changed; review the backport');
  return (
    source + '\n[lints.rust]\nunused_parens = "allow"\nmismatched_lifetime_syntaxes = "allow"\n'
  );
}

async function files(directory, prefix = '') {
  const result = [];
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    const relative = prefix + entry.name;
    if (entry.isDirectory())
      result.push(...(await files(path.join(directory, entry.name), `${relative}/`)));
    else {
      assert.ok(entry.isFile(), `Unsupported package entry: ${relative}`);
      result.push(relative);
    }
  }
  return result.sort();
}

export async function upstream() {
  const cache = path.join(root, 'target', 'glib-backport-upstream');
  await mkdir(cache, { recursive: true });
  const archivePath = path.join(cache, `${packageName}.crate`);
  let archive;
  let downloaded = false;
  try {
    archive = await readFile(archivePath);
  } catch (error) {
    if (error.code !== 'ENOENT') throw error;
    const response = await fetch(provenance.archive, { signal: AbortSignal.timeout(60000) });
    assert.ok(response.ok, `Upstream download failed: ${response.status}`);
    archive = Buffer.from(await response.arrayBuffer());
    downloaded = true;
  }
  assert.equal(sha256(archive), provenance.sha256, 'Upstream archive checksum mismatch');
  if (downloaded)
    await writeFile(archivePath, archive, { flag: 'wx' }).catch(async (error) => {
      if (error.code !== 'EEXIST') throw error;
      assert.equal(sha256(await readFile(archivePath)), provenance.sha256);
    });
  const entries = execFileSync('tar', ['-tzf', archivePath], { encoding: 'utf8' })
    .trim()
    .split(/\r?\n/);
  assert.ok(
    entries.every((name) => name.startsWith(`${packageName}/`) && !name.split('/').includes('..')),
  );
  const extraction = await mkdtemp(path.join(cache, 'source-'));
  execFileSync('tar', ['-xzf', archivePath, '-C', extraction]);
  return path.join(extraction, packageName);
}

export async function verifyTrees(original, actual) {
  const expectedFiles = await files(original);
  assert.deepEqual(
    await files(actual),
    expectedFiles,
    'Vendored package file set differs from upstream',
  );
  for (const relative of expectedFiles) {
    let expected = await readFile(path.join(original, relative));
    if (relative === 'src/variant_iter.rs')
      expected = Buffer.from(patchIterator(expected.toString('utf8')));
    if (relative === 'Cargo.toml') expected = Buffer.from(patchManifest(expected.toString('utf8')));
    assert.equal(
      sha256(await readFile(path.join(actual, relative))),
      sha256(expected),
      `Unexpected modification: ${relative}`,
    );
  }
  return expectedFiles.length;
}

async function main() {
  const original = await upstream();
  if (process.argv[2] === 'import') {
    await mkdir(path.dirname(vendor), { recursive: true });
    await mkdir(vendor, { recursive: true });
    assert.equal((await readdir(vendor)).length, 0, 'Refusing to overwrite a maintained package');
    for (const entry of await readdir(original)) {
      await cp(path.join(original, entry), path.join(vendor, entry), {
        recursive: true,
        force: false,
        errorOnExist: true,
      });
    }
    console.log(`Imported pristine ${packageName}; apply the reviewed two-line patch next.`);
    return;
  }
  assert.equal(process.argv[2], 'verify');
  const count = await verifyTrees(original, vendor);
  const manifest = await readFile(path.join(root, 'Cargo.toml'), 'utf8');
  assert.match(manifest, /\[patch\.crates-io\]\s+glib = \{ path = "vendor\/glib-0\.18\.5" \}/);
  const lock = await readFile(path.join(root, 'Cargo.lock'), 'utf8');
  const glib = lock.split('[[package]]').filter((entry) => /^name = "glib"$/m.test(entry));
  assert.equal(glib.length, 1, 'Multiple or missing glib packages');
  assert.match(glib[0], /^version = "0\.18\.5"$/m);
  assert.doesNotMatch(
    glib[0],
    /^(source|checksum) = /m,
    'glib must resolve to the local patched source',
  );
  console.log(
    `Verified ${count} upstream files: exact ${provenance.advisory} source fix and two style-lint compatibility settings only.`,
  );
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url))
  await main();
