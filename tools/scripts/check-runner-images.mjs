import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { load } from 'js-yaml';

const PLATFORMS = ['windows', 'macos'];
const RUNNER_LABEL = /\b(windows|macos)-(?:latest|\d+(?:\.\d+)?(?:-[a-z0-9]+)*)\b/gi;

export function collectRunnerImageErrors({ policy, workflows }) {
  const errors = validatePolicy(policy);
  if (errors.length > 0) return errors;

  const allowed = new Map(PLATFORMS.map((platform) => [platform, new Set(policy.pins[platform])]));
  const seen = new Map(PLATFORMS.map((platform) => [platform, new Set()]));

  for (const [file, source] of workflows) {
    let workflow;
    try {
      workflow = load(source);
    } catch (error) {
      errors.push(`${file}: invalid workflow YAML (${error.message})`);
      continue;
    }

    for (const value of scalarStrings(workflow)) {
      for (const match of value.matchAll(RUNNER_LABEL)) {
        const platform = match[1].toLowerCase();
        const label = match[0].toLowerCase();
        seen.get(platform).add(label);

        if (label.endsWith('-latest')) {
          errors.push(`${file}: floating runner label is forbidden: ${label}`);
        } else if (!allowed.get(platform).has(label)) {
          errors.push(
            `${file}: ${label} is not reviewed; allowed ${platform} labels: ${[
              ...allowed.get(platform),
            ].join(', ')}`,
          );
        }
      }
    }
  }

  for (const platform of PLATFORMS) {
    for (const label of allowed.get(platform)) {
      if (!seen.get(platform).has(label)) {
        errors.push(`reviewed runner label is unused by workflows: ${label}`);
      }
    }
  }

  return [...new Set(errors)];
}

export function verifyPinnedRunnerImages(rootDir) {
  const policyPath = path.join(rootDir, 'tools/ci/runner-images.json');
  const workflowDir = path.join(rootDir, '.github/workflows');
  const policy = JSON.parse(fs.readFileSync(policyPath, 'utf8'));
  const workflows = fs
    .readdirSync(workflowDir)
    .filter((file) => /\.ya?ml$/i.test(file))
    .sort()
    .map((file) => [file, fs.readFileSync(path.join(workflowDir, file), 'utf8')]);
  const errors = collectRunnerImageErrors({ policy, workflows });
  if (errors.length > 0) {
    throw new Error(`Invalid GitHub runner image policy:\n- ${errors.join('\n- ')}`);
  }
  return policy;
}

function validatePolicy(policy) {
  const errors = [];
  if (policy?.schemaVersion !== 1) errors.push('schemaVersion must be 1');
  if (policy?.source !== 'https://github.com/actions/runner-images#available-images') {
    errors.push('source must point to the official GitHub runner image list');
  }
  if (!/^\d{4}-\d{2}-\d{2}$/.test(policy?.reviewedAt ?? '')) {
    errors.push('reviewedAt must use YYYY-MM-DD');
  }

  for (const platform of PLATFORMS) {
    const labels = policy?.pins?.[platform];
    if (!Array.isArray(labels) || labels.length === 0) {
      errors.push(`pins.${platform} must contain at least one reviewed label`);
      continue;
    }
    for (const label of labels) {
      if (
        typeof label !== 'string' ||
        !new RegExp(`^${platform}-\\d+(?:\\.\\d+)?(?:-[a-z0-9]+)*$`).test(label)
      ) {
        errors.push(`pins.${platform} contains an invalid concrete label: ${String(label)}`);
      }
    }
    if (new Set(labels).size !== labels.length) {
      errors.push(`pins.${platform} must not contain duplicate labels`);
    }
  }
  return errors;
}

function* scalarStrings(value) {
  if (typeof value === 'string') {
    yield value;
  } else if (Array.isArray(value)) {
    for (const item of value) yield* scalarStrings(item);
  } else if (value && typeof value === 'object') {
    for (const item of Object.values(value)) yield* scalarStrings(item);
  }
}

const currentFile = fileURLToPath(import.meta.url);
if (path.resolve(process.argv[1] ?? '') === currentFile) {
  const rootDir = path.resolve(path.dirname(currentFile), '../..');
  try {
    const policy = verifyPinnedRunnerImages(rootDir);
    process.stdout.write(
      `Windows and macOS workflows use reviewed runner images (${policy.reviewedAt}).\n`,
    );
  } catch (error) {
    process.stderr.write(`${error.message}\n`);
    process.exitCode = 1;
  }
}
