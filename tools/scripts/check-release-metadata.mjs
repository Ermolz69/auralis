import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

export function collectMetadataErrors({
  workspaceCargo,
  memberCargos,
  rootPackage,
  desktopPackage,
  tauriConfig,
}) {
  const errors = [];
  const workspacePackage = section(workspaceCargo, 'workspace.package');
  const workspaceVersion = stringValue(workspacePackage, 'version');
  const workspaceEdition = stringValue(workspacePackage, 'edition');
  const workspaceAuthors = arrayValues(workspacePackage, 'authors');

  if (!workspaceVersion) errors.push('workspace.package.version must be declared');
  if (workspaceEdition !== '2024') errors.push('workspace.package.edition must be 2024');
  if (workspaceAuthors.length === 0 || workspaceAuthors.some((author) => author === 'you')) {
    errors.push('workspace.package.authors must identify the project maintainers');
  }
  for (const [member, cargo] of memberCargos) {
    const packageSection = section(cargo, 'package');
    for (const inherited of ['version', 'edition', 'authors']) {
      if (!new RegExp(`^${inherited}\\.workspace\\s*=\\s*true$`, 'm').test(packageSection)) {
        errors.push(`${member} must inherit package ${inherited} from the workspace`);
      }
    }
  }

  if (rootPackage.version !== workspaceVersion) {
    errors.push('root package.json version must equal workspace.package.version');
  }
  if (desktopPackage.version !== workspaceVersion) {
    errors.push('desktop package.json version must equal workspace.package.version');
  }
  if (tauriConfig.version !== '../package.json') {
    errors.push('Tauri must read its version from the root package.json');
  }
  if (typeof tauriConfig.identifier !== 'string' || tauriConfig.identifier.endsWith('.app')) {
    errors.push('Tauri identifier must be stable and must not end with .app');
  }
  return errors;
}

export function verifyReleaseMetadata(rootDir) {
  const workspaceCargo = fs.readFileSync(path.join(rootDir, 'Cargo.toml'), 'utf8');
  const members = workspaceMembers(workspaceCargo);
  const memberCargos = members.map((member) => [
    member,
    fs.readFileSync(path.join(rootDir, member, 'Cargo.toml'), 'utf8'),
  ]);
  const errors = collectMetadataErrors({
    workspaceCargo,
    memberCargos,
    rootPackage: readJson(path.join(rootDir, 'package.json')),
    desktopPackage: readJson(path.join(rootDir, 'apps/desktop/package.json')),
    tauriConfig: readJson(path.join(rootDir, 'src-tauri/tauri.conf.json')),
  });
  if (errors.length > 0) throw new Error(`Invalid release metadata:\n- ${errors.join('\n- ')}`);
  verifyReleaseWorkflow(rootDir);
}

export function verifyReleaseWorkflow(rootDir) {
  const workflow = fs.readFileSync(path.join(rootDir, '.github/workflows/release.yml'), 'utf8');
  for (const requirement of [
    'task release:tag:verify',
    'task release:signing:preflight',
    "AURALIS_REQUIRE_SIGNING: 'true'",
    'task release:signature:verify',
    'task media:bundle:verify',
    'task release:smoke:install-launch',
    'TAURI_SIGNING_PRIVATE_KEY:',
    'TAURI_SIGNING_PRIVATE_KEY_PASSWORD:',
    'AURALIS_UPDATER_PUBLIC_KEY:',
    'src-tauri/tauri.release.conf.json',
    '*.AppImage.sig',
    '*-setup.exe.sig',
    '*.app.tar.gz.sig',
  ]) {
    if (!workflow.includes(requirement)) {
      throw new Error(`Production release workflow is missing: ${requirement}`);
    }
  }
  if (workflow.includes('OS CODE SIGNING PLACEHOLDERS')) {
    throw new Error('Production release workflow still contains signing placeholders');
  }
  assertAtomicReleaseWorkflow(workflow);
  const windowsConfig = readJson(path.join(rootDir, 'src-tauri/tauri.windows.conf.json'));
  if (windowsConfig.bundle?.windows?.signCommand !== 'node ../tools/release/sign-windows.mjs %1') {
    throw new Error('Windows bundle must use the audited release signing command');
  }
  if (windowsConfig.bundle?.windows?.nsis?.installMode !== 'currentUser') {
    throw new Error('Windows NSIS smoke package must use the non-elevated currentUser mode');
  }
  const tauriConfig = readJson(path.join(rootDir, 'src-tauri/tauri.conf.json'));
  const updaterEndpoint =
    'https://github.com/Ermolz69/auralis/releases/latest/download/latest.json';
  if (!tauriConfig.plugins?.updater?.endpoints?.includes(updaterEndpoint)) {
    throw new Error('Tauri updater must use the static GitHub Releases latest.json endpoint');
  }
  const releaseConfig = readJson(path.join(rootDir, 'src-tauri/tauri.release.conf.json'));
  if (releaseConfig.bundle?.createUpdaterArtifacts !== true) {
    throw new Error('Production bundles must create signed Tauri updater artifacts');
  }
  const rustEntry = fs.readFileSync(path.join(rootDir, 'src-tauri/src/lib.rs'), 'utf8');
  if (
    !rustEntry.includes('option_env!("AURALIS_UPDATER_PUBLIC_KEY")') ||
    !rustEntry.includes('.pubkey(public_key)')
  ) {
    throw new Error('Production updater public key must be embedded from the release environment');
  }
}

export function assertAtomicReleaseWorkflow(workflow) {
  for (const forbidden of ['tagName:', 'releaseName:', 'releaseBody:', 'releaseDraft:']) {
    if (workflow.includes(forbidden)) {
      throw new Error(`Matrix build must not create a GitHub Release: ${forbidden}`);
    }
  }
  const buildJob = workflowJob(workflow, 'build');
  const publishJob = workflowJob(workflow, 'publish');
  if (!buildJob || !publishJob) throw new Error('Release workflow requires build and publish jobs');
  if (buildJob.includes('contents: write') || buildJob.includes('GITHUB_TOKEN')) {
    throw new Error('Matrix build must have read-only repository access');
  }
  if (!publishJob.includes('needs: build')) {
    throw new Error('Publish job must wait for the complete build matrix');
  }
  if (!publishJob.includes('contents: write')) {
    throw new Error('Only the publish job may receive release write permission');
  }
  if ((workflow.match(/contents:\s*write/g) ?? []).length !== 1) {
    throw new Error('Release write permission must appear exactly once');
  }
  const orderedMarkers = [
    'uses: tauri-apps/tauri-action@v1',
    'task media:bundle:verify',
    'task release:signature:verify',
    'task release:smoke:install-launch',
    'task release:assets:validate',
    'uses: actions/upload-artifact@v4',
    'uses: actions/download-artifact@v5',
    'task release:assets:prepare-publish',
    'gh release create',
  ];
  let previous = -1;
  for (const marker of orderedMarkers) {
    const index = workflow.indexOf(marker);
    if (index < 0) throw new Error(`Atomic release workflow is missing: ${marker}`);
    if (index <= previous)
      throw new Error(`Atomic release workflow step is out of order: ${marker}`);
    previous = index;
  }
}

function workflowJob(workflow, name) {
  return workflow.match(
    new RegExp(`^  ${name}:\\s*$([\\s\\S]*?)(?=^  [a-zA-Z0-9_-]+:\\s*$|(?![\\s\\S]))`, 'm'),
  )?.[1];
}

function workspaceMembers(cargo) {
  const members = section(cargo, 'workspace').match(/members\s*=\s*\[([\s\S]*?)\]/)?.[1];
  if (!members) throw new Error('Cannot find Cargo workspace members');
  return [...members.matchAll(/"([^"]+)"/g)].map((match) => match[1]);
}

function section(toml, name) {
  return (
    toml.match(
      new RegExp(`^\\[${name.replace('.', '\\.')}\\]\\s*$([\\s\\S]*?)(?=^\\[|(?![\\s\\S]))`, 'm'),
    )?.[1] ?? ''
  );
}

function stringValue(tomlSection, key) {
  return tomlSection.match(new RegExp(`^${key}\\s*=\\s*"([^"]+)"`, 'm'))?.[1];
}

function arrayValues(tomlSection, key) {
  const values = tomlSection.match(new RegExp(`^${key}\\s*=\\s*\\[([^\\]]*)\\]`, 'm'))?.[1];
  return values ? [...values.matchAll(/"([^"]+)"/g)].map((match) => match[1]) : [];
}

function readJson(file) {
  return JSON.parse(fs.readFileSync(file, 'utf8'));
}

const currentFile = fileURLToPath(import.meta.url);
if (path.resolve(process.argv[1] ?? '') === currentFile) {
  const rootDir = path.resolve(path.dirname(currentFile), '../..');
  try {
    verifyReleaseMetadata(rootDir);
    process.stdout.write('Rust, frontend and Tauri release metadata are synchronized.\n');
  } catch (error) {
    process.stderr.write(`${error.message}\n`);
    process.exitCode = 1;
  }
}
