import fs from 'node:fs';
import os from 'node:os';

export const toolNames = ['ffmpeg', 'ffprobe', 'yt-dlp'];
export const releaseTargets = [
  'x86_64-pc-windows-msvc',
  'x86_64-unknown-linux-gnu',
  'aarch64-unknown-linux-gnu',
  'x86_64-apple-darwin',
  'aarch64-apple-darwin',
];

const platformTargets = new Map([
  ['win32:x64', 'x86_64-pc-windows-msvc'],
  ['linux:x64', 'x86_64-unknown-linux-gnu'],
  ['linux:arm64', 'aarch64-unknown-linux-gnu'],
  ['darwin:x64', 'x86_64-apple-darwin'],
  ['darwin:arm64', 'aarch64-apple-darwin'],
]);

const trustedDownloadHosts = new Set(['github.com', 'raw.githubusercontent.com']);
const sha256Pattern = /^[a-f0-9]{64}$/;
const revisionPattern = /^[a-f0-9]{40}$/;
const outputPattern = /^[A-Z0-9._-]+$/i;

export function loadManifest(manifestPath) {
  return validateManifest(JSON.parse(fs.readFileSync(manifestPath, 'utf8')));
}

export function resolveTarget(explicitTarget = process.env.AURALIS_TARGET_TRIPLE) {
  if (explicitTarget) return explicitTarget;
  const target = platformTargets.get(`${os.platform()}:${os.arch()}`);
  if (!target) throw new Error(`Unsupported media-tools host: ${os.platform()} ${os.arch()}`);
  return target;
}

export function validateManifest(manifest) {
  const errors = [];
  if (manifest?.schemaVersion !== 1) errors.push('schemaVersion must be 1');
  validateLicenses(manifest?.licenses, errors);
  validateTools(manifest?.tools, manifest?.licenses, errors);
  validateTargets(manifest?.targets, errors);

  if (errors.length > 0) {
    throw new Error(`Invalid media-tools manifest:\n- ${errors.join('\n- ')}`);
  }
  return manifest;
}

export function targetAssets(manifest, target) {
  const assets = manifest.targets[target];
  if (!assets) throw new Error(`Unsupported media-tools target: ${target}`);
  return toolNames.map((name) => ({ name, tool: manifest.tools[name], ...assets[name] }));
}

function validateLicenses(licenses, errors) {
  if (!licenses || typeof licenses !== 'object') {
    errors.push('licenses must be an object');
    return;
  }
  for (const [name, license] of Object.entries(licenses)) {
    validateUrl(license?.url, `license ${name}`, errors);
    validateSha(license?.sha256, `license ${name}`, errors);
    validateOutput(license?.output, `license ${name}`, errors);
    if (!license?.spdx) errors.push(`license ${name} must declare an SPDX identifier`);
  }
}

function validateTools(tools, licenses, errors) {
  if (!tools || typeof tools !== 'object') {
    errors.push('tools must be an object');
    return;
  }
  for (const name of toolNames) {
    const tool = tools[name];
    if (!tool) {
      errors.push(`tool ${name} is missing`);
      continue;
    }
    if (!tool.version) errors.push(`tool ${name} must declare a version`);
    if (!Array.isArray(tool.versionArgs) || tool.versionArgs.length === 0) {
      errors.push(`tool ${name} must declare versionArgs`);
    }
    try {
      new RegExp(tool.versionPattern);
    } catch {
      errors.push(`tool ${name} has an invalid versionPattern`);
    }
    if (typeof tool.license !== 'string' || !licenses?.[tool.license]) {
      errors.push(`tool ${name} must reference a declared license`);
    }
    validateUrl(tool.sourceUrl, `tool ${name} source`, errors);
    validateRevision(tool.sourceRevision, `tool ${name} source`, errors);
    if (name !== 'yt-dlp') {
      validateUrl(tool.buildUrl, `tool ${name} build`, errors);
      validateRevision(tool.buildRevision, `tool ${name} build`, errors);
    }
  }

}

function validateTargets(targets, errors) {
  if (!targets || typeof targets !== 'object') {
    errors.push('targets must be an object');
    return;
  }
  for (const required of releaseTargets) {
    if (!targets[required]) errors.push(`release target ${required} is missing`);
  }
  for (const [target, assets] of Object.entries(targets)) {
    for (const name of toolNames) {
      const asset = assets?.[name];
      if (!asset) {
        errors.push(`${target} is missing ${name}`);
        continue;
      }
      validateUrl(asset.url, `${target} ${name}`, errors);
      validateSha(asset.sha256, `${target} ${name}`, errors);
      validateOutput(asset.output, `${target} ${name}`, errors);
      if (!asset.output.startsWith(`${name}-${target}`)) {
        errors.push(`${target} ${name} output must include its target triple`);
      }
    }
  }
}

function validateUrl(value, label, errors) {
  try {
    const url = new URL(value);
    if (url.protocol !== 'https:' || !trustedDownloadHosts.has(url.hostname)) {
      errors.push(`${label} must use a trusted HTTPS download host`);
    }
  } catch {
    errors.push(`${label} has an invalid URL`);
  }
}

function validateSha(value, label, errors) {
  if (!sha256Pattern.test(value ?? '')) errors.push(`${label} must declare a SHA-256 digest`);
}

function validateRevision(value, label, errors) {
  if (!revisionPattern.test(value ?? '')) errors.push(`${label} must pin a source revision`);
}

function validateOutput(value, label, errors) {
  if (!outputPattern.test(value ?? '')) errors.push(`${label} has an unsafe output filename`);
}
