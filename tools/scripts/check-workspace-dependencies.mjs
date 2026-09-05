import { readFile } from 'node:fs/promises';
import path from 'node:path';
import { pathToFileURL } from 'node:url';

const dependencySectionPattern =
  /^(?:target\.(?:'[^']+'|"[^"]+")\.)?(?:build-|dev-)?dependencies$/;

function parseSectionHeader(line) {
  const match = line.trim().match(/^\[([^\]]+)]$/);
  return match?.[1] ?? null;
}

function braceDelta(value) {
  return [...value].reduce((depth, character) => {
    if (character === '{') return depth + 1;
    if (character === '}') return depth - 1;
    return depth;
  }, 0);
}

function parseDependencyDeclarations(manifest) {
  const declarations = [];
  const lines = manifest.split(/\r?\n/);
  let section = null;

  for (let index = 0; index < lines.length; index += 1) {
    const header = parseSectionHeader(lines[index]);
    if (header !== null) {
      section = header;
      continue;
    }

    if (!dependencySectionPattern.test(section ?? '')) continue;

    const match = lines[index].match(
      /^\s*([A-Za-z0-9_-]+)(\.workspace)?\s*=\s*(.+)$/,
    );
    if (!match) continue;

    const [, name, workspaceSuffix, initialValue] = match;
    let value = initialValue;
    let depth = braceDelta(initialValue);

    while (depth > 0 && index + 1 < lines.length) {
      index += 1;
      value += `\n${lines[index]}`;
      depth += braceDelta(lines[index]);
    }

    declarations.push({
      name,
      section,
      usesWorkspace:
        workspaceSuffix === '.workspace' || /\bworkspace\s*=\s*true\b/.test(value),
      usesPath: /\bpath\s*=/.test(value),
    });
  }

  return declarations;
}

export function parseWorkspaceMembers(rootManifest) {
  const membersBlock = rootManifest.match(/\bmembers\s*=\s*\[([\s\S]*?)]/);
  if (!membersBlock) return [];

  return [...membersBlock[1].matchAll(/"([^"]+)"/g)].map((match) => match[1]);
}

export function parseCentralizedDependencies(rootManifest) {
  const lines = rootManifest.split(/\r?\n/);
  const dependencies = new Set();
  let inWorkspaceDependencies = false;

  for (const line of lines) {
    const header = parseSectionHeader(line);
    if (header !== null) {
      inWorkspaceDependencies = header === 'workspace.dependencies';
      continue;
    }

    if (!inWorkspaceDependencies) continue;
    const match = line.match(/^\s*([A-Za-z0-9_-]+)\s*=/);
    if (match) dependencies.add(match[1]);
  }

  return dependencies;
}

export function collectWorkspaceDependencyErrors({ rootManifest, memberManifests }) {
  const errors = [];
  const centralized = parseCentralizedDependencies(rootManifest);
  const unmanagedExternalUsage = new Map();

  for (const [manifestPath, manifest] of memberManifests) {
    for (const dependency of parseDependencyDeclarations(manifest)) {
      if (dependency.usesWorkspace && !centralized.has(dependency.name)) {
        errors.push(
          `${manifestPath}: ${dependency.name} inherits from the workspace but is missing from [workspace.dependencies]`,
        );
        continue;
      }

      if (centralized.has(dependency.name) && !dependency.usesWorkspace) {
        errors.push(
          `${manifestPath}: ${dependency.name} must use workspace = true instead of a local version`,
        );
        continue;
      }

      if (dependency.usesWorkspace || dependency.usesPath) continue;

      const manifests = unmanagedExternalUsage.get(dependency.name) ?? new Set();
      manifests.add(manifestPath);
      unmanagedExternalUsage.set(dependency.name, manifests);
    }
  }

  for (const [dependency, manifests] of unmanagedExternalUsage) {
    if (manifests.size < 2) continue;
    errors.push(
      `${dependency} is versioned independently in multiple manifests: ${[...manifests].sort().join(', ')}`,
    );
  }

  return errors.sort();
}

export async function checkWorkspaceDependencies(rootDir = process.cwd()) {
  const rootManifestPath = path.join(rootDir, 'Cargo.toml');
  const rootManifest = await readFile(rootManifestPath, 'utf8');
  const memberManifests = new Map();

  for (const member of parseWorkspaceMembers(rootManifest)) {
    const manifestPath = path.join(member, 'Cargo.toml');
    memberManifests.set(
      manifestPath.replaceAll('\\', '/'),
      await readFile(path.join(rootDir, manifestPath), 'utf8'),
    );
  }

  return collectWorkspaceDependencyErrors({ rootManifest, memberManifests });
}

const isDirectRun =
  process.argv[1] && import.meta.url === pathToFileURL(path.resolve(process.argv[1])).href;

if (isDirectRun) {
  const errors = await checkWorkspaceDependencies();
  if (errors.length > 0) {
    console.error(`Cargo workspace dependency policy failed:\n- ${errors.join('\n- ')}`);
    process.exitCode = 1;
  } else {
    console.log('Cargo workspace dependency policy passed.');
  }
}
