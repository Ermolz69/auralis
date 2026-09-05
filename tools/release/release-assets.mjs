import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const PLATFORM_RULES = {
  windows: [
    ['Windows MSI', (file) => file.toLowerCase().endsWith('.msi')],
    ['Windows MSI updater signature', (file) => file.toLowerCase().endsWith('.msi.sig')],
    ['Windows NSIS installer', (file) => file.toLowerCase().endsWith('-setup.exe')],
    ['Windows NSIS updater signature', (file) => file.toLowerCase().endsWith('-setup.exe.sig')],
  ],
  macos: [
    ['macOS DMG', (file) => file.toLowerCase().endsWith('.dmg')],
    ['macOS updater bundle', (file) => file.toLowerCase().endsWith('.app.tar.gz')],
    ['macOS updater signature', (file) => file.toLowerCase().endsWith('.app.tar.gz.sig')],
  ],
  linux: [
    ['Linux AppImage', (file) => file.toLowerCase().endsWith('.appimage')],
    ['Linux updater signature', (file) => file.toLowerCase().endsWith('.appimage.sig')],
    ['Debian package', (file) => file.toLowerCase().endsWith('.deb')],
    ['RPM package', (file) => file.toLowerCase().endsWith('.rpm')],
  ],
};

export function collectReleaseAssets(root, platform) {
  const rules = selectedRules(platform);
  const files = walkFiles(root);
  const assets = rules.map(([label, matches]) => {
    const candidates = files.filter(matches);
    if (candidates.length !== 1) {
      throw new Error(`Expected exactly one ${label}, found ${candidates.length} under ${root}`);
    }
    return candidates[0];
  });
  const basenames = assets.map((file) => path.basename(file).toLowerCase());
  if (new Set(basenames).size !== basenames.length) {
    throw new Error('Release asset basenames must be unique across platforms');
  }
  return assets.sort((left, right) => path.basename(left).localeCompare(path.basename(right)));
}

export function buildUpdaterManifest(assets, { version, tag, repository, notes }) {
  validateMetadata({ version, tag, repository });
  const bySuffix = (suffix) => {
    const matches = assets.filter((file) => file.toLowerCase().endsWith(suffix));
    if (matches.length !== 1) throw new Error(`Expected one updater asset ending in ${suffix}`);
    return matches[0];
  };
  const platform = (bundleSuffix, signatureSuffix) => {
    const bundle = bySuffix(bundleSuffix);
    const signature = fs.readFileSync(bySuffix(signatureSuffix), 'utf8').trim();
    if (!signature) throw new Error(`Updater signature for ${path.basename(bundle)} is empty`);
    return {
      signature,
      url: `https://github.com/${repository}/releases/download/${tag}/${encodeURIComponent(path.basename(bundle))}`,
    };
  };
  return {
    version,
    notes: notes?.trim() || `Auralis ${version}`,
    platforms: {
      'linux-x86_64': platform('.appimage', '.appimage.sig'),
      'windows-x86_64': platform('-setup.exe', '-setup.exe.sig'),
      'darwin-aarch64': platform('.app.tar.gz', '.app.tar.gz.sig'),
    },
  };
}

export function prepareReleaseAssets(inputRoot, outputRoot, metadata) {
  const assets = collectReleaseAssets(inputRoot, 'all');
  const manifest = buildUpdaterManifest(assets, metadata);
  if (fs.existsSync(outputRoot)) {
    throw new Error(`Release publish directory already exists: ${outputRoot}`);
  }
  fs.mkdirSync(outputRoot, { recursive: true });
  const checksums = [];
  for (const source of assets) {
    const basename = path.basename(source);
    fs.copyFileSync(source, path.join(outputRoot, basename), fs.constants.COPYFILE_EXCL);
    checksums.push(`${sha256(source)}  ${basename}`);
  }
  const manifestFile = path.join(outputRoot, 'latest.json');
  fs.writeFileSync(manifestFile, `${JSON.stringify(manifest, null, 2)}\n`, { flag: 'wx' });
  checksums.push(`${sha256(manifestFile)}  latest.json`);
  fs.writeFileSync(path.join(outputRoot, 'SHA256SUMS.txt'), `${checksums.join('\n')}\n`, {
    flag: 'wx',
  });
  return [...assets.map((file) => path.basename(file)), 'latest.json'];
}

function validateMetadata({ version, tag, repository }) {
  if (!/^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/.test(version ?? '')) {
    throw new Error('Updater version must be a valid SemVer value');
  }
  if (tag !== `app-v${version}`) {
    throw new Error(`Release tag ${tag} must equal app-v${version}`);
  }
  if (!/^[^/\s]+\/[^/\s]+$/.test(repository ?? '')) {
    throw new Error('GitHub repository must use the owner/name format');
  }
}

function selectedRules(platform) {
  if (platform === 'all') return Object.values(PLATFORM_RULES).flat();
  const rules = PLATFORM_RULES[platform];
  if (!rules) throw new Error(`Unsupported release asset platform: ${platform}`);
  return rules;
}

function walkFiles(root, files = []) {
  if (!fs.existsSync(root)) return files;
  for (const entry of fs.readdirSync(root, { withFileTypes: true })) {
    const candidate = path.join(root, entry.name);
    if (entry.isDirectory()) walkFiles(candidate, files);
    else if (entry.isFile()) files.push(candidate);
  }
  return files;
}

function sha256(file) {
  return crypto.createHash('sha256').update(fs.readFileSync(file)).digest('hex');
}

function argument(name) {
  const prefix = `--${name}=`;
  return process.argv.find((entry) => entry.startsWith(prefix))?.slice(prefix.length);
}

const currentFile = fileURLToPath(import.meta.url);
if (path.resolve(process.argv[1] ?? '') === currentFile) {
  try {
    const platform = argument('platform');
    const root = argument('root');
    const output = argument('output');
    if (!platform || !root) throw new Error('--platform and --root are required');
    const resolvedRoot = path.resolve(root);
    if (output) {
      if (platform !== 'all') throw new Error('--output requires --platform=all');
      const workspaceRoot = path.resolve(path.dirname(currentFile), '../..');
      const { version } = JSON.parse(
        fs.readFileSync(path.join(workspaceRoot, 'package.json'), 'utf8'),
      );
      const assets = prepareReleaseAssets(resolvedRoot, path.resolve(output), {
        version,
        tag: argument('tag') ?? process.env.GITHUB_REF_NAME,
        repository: argument('repository') ?? process.env.GITHUB_REPOSITORY,
        notes: process.env.AURALIS_RELEASE_NOTES,
      });
      process.stdout.write(`Prepared ${assets.length} verified release assets and checksums.\n`);
    } else {
      const assets = collectReleaseAssets(resolvedRoot, platform);
      process.stdout.write(`Validated ${assets.length} ${platform} release asset(s).\n`);
    }
  } catch (error) {
    process.stderr.write(`${error.message}\n`);
    process.exitCode = 1;
  }
}
