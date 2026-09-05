import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { verifyBuiltBundle } from './bundle-verify.mjs';
import { loadManifest, resolveTarget } from './manifest.mjs';
import { prepareMediaTools } from './prepare.mjs';
import { verifyRepositoryPolicy } from './repository-policy.mjs';
import { verifyPrepared } from './verify.mjs';

const currentFile = fileURLToPath(import.meta.url);
const rootDir = path.resolve(path.dirname(currentFile), '../..');
const manifestPath = path.join(rootDir, 'tools/media-tools/manifest.json');
const binariesDir = path.join(rootDir, 'src-tauri/binaries');

async function main() {
  const command = process.argv[2] ?? 'check';
  const targetArgument = process.argv.find((argument) => argument.startsWith('--target='));
  const target = resolveTarget(targetArgument?.slice('--target='.length));
  const manifest = loadManifest(manifestPath);

  if (command === 'check') {
    verifyRepositoryPolicy(rootDir);
    process.stdout.write(`Media-tools manifest and repository wiring verified for ${target}.\n`);
    return;
  }
  if (command === 'prepare') {
    await prepareMediaTools({ manifest, target, binariesDir });
    process.stdout.write(`Prepared verified media tools for ${target}.\n`);
    return;
  }
  if (command === 'verify') {
    await verifyPrepared({ manifest, target, binariesDir, execute: true });
    process.stdout.write(`Verified bundled media tools for ${target}.\n`);
    return;
  }
  if (command === 'verify-bundle') {
    await verifyBuiltBundle({ rootDir, manifest, target });
    process.stdout.write(`Verified media tools inside the ${target} installer bundle.\n`);
    return;
  }
  throw new Error(`Unknown media-tools command: ${command}`);
}

main().catch((error) => {
  process.stderr.write(`${error.message}\n`);
  process.exitCode = 1;
});
