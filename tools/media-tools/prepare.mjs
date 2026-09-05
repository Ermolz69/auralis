import path from 'node:path';
import { downloadVerified, writeIfChanged } from './integrity.mjs';
import { targetAssets } from './manifest.mjs';
import { renderNotices, renderProvenance } from './provenance.mjs';
import { prepareStagingDirectory } from './staged-files.mjs';
import { verifyPrepared } from './verify.mjs';

export async function prepareMediaTools({ manifest, target, binariesDir }) {
  await prepareStagingDirectory({ manifest, target, binariesDir });

  for (const asset of targetAssets(manifest, target)) {
    const state = await downloadVerified({
      url: asset.url,
      sha256: asset.sha256,
      destination: path.join(binariesDir, asset.output),
      executable: true,
    });
    process.stdout.write(`${asset.name} ${asset.tool.version}: ${state}\n`);
  }

  for (const [name, license] of Object.entries(manifest.licenses)) {
    const state = await downloadVerified({
      url: license.url,
      sha256: license.sha256,
      destination: path.join(binariesDir, license.output),
    });
    process.stdout.write(`${name} license: ${state}\n`);
  }

  await writeIfChanged(
    path.join(binariesDir, 'media-tools-provenance.json'),
    renderProvenance(manifest, target),
  );
  await writeIfChanged(
    path.join(binariesDir, 'THIRD-PARTY-NOTICES.txt'),
    renderNotices(manifest, target),
  );
  await verifyPrepared({ manifest, target, binariesDir, execute: true });
}
