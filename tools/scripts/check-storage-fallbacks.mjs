import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const defaultRootDir = path.resolve(__dirname, '../../');

const SCAN_DIRS = [
  'crates/application/src',
  'crates/jobs/src',
  'crates/adapters-storage/src',
  'crates/adapters-tauri/src',
  'crates/adapters-ytdlp/src',
  'crates/adapters-ffmpeg/src',
  'crates/adapters-model/src',
  'crates/ports/src',
  'src-tauri/src',
];

const ARTIFACTS_JSON_ALLOWLIST = [
  'crates/adapters-storage/src/sqlite/migrations_runtime/backfill_artifacts.rs',
  'crates/adapters-storage/src/sqlite/migrations_runtime/tests.rs',
  'crates/adapters-storage/src/sqlite/preflight/tests.rs',
  'crates/adapters-storage/src/sqlite/preflight/inspector.rs',
  'crates/adapters-storage/src/sqlite/preflight/state_machine.rs',
];

const TEST_PATTERNS = [/(^|\/)tests\.rs$/, /(^|\/)tests\//, /_tests\.rs$/];

function normalizeRelative(rootDir, filePath) {
  return path.relative(rootDir, filePath).replace(/\\/g, '/');
}

function walkSync(dir, files = []) {
  if (!fs.existsSync(dir)) return files;

  for (const entry of fs.readdirSync(dir).sort()) {
    const fullPath = path.join(dir, entry);
    if (fs.statSync(fullPath).isDirectory()) {
      walkSync(fullPath, files);
    } else if (fullPath.endsWith('.rs')) {
      files.push(fullPath);
    }
  }

  return files;
}

function isTestFile(relativePath) {
  return TEST_PATTERNS.some((pattern) => pattern.test(relativePath));
}

function isAllowedArtifactsJsonPath(relativePath) {
  return ARTIFACTS_JSON_ALLOWLIST.includes(relativePath);
}

function hasInlineAllow(line) {
  return line.includes('allow-fallback');
}

function isComment(line) {
  return line.trim().startsWith('//');
}

function classifyLine(line, relativePath) {
  if (isComment(line) || hasInlineAllow(line)) return null;

  if (line.includes('.unwrap_or_default()') && !line.includes('"')) {
    return 'unwrap_or_default storage fallback';
  }

  if (line.match(/let\s+_\s*=\s*.*\.(commit|rollback)\(\)/)) {
    return 'ignored transaction result';
  }

  if (
    line.match(/let\s+_\s*=\s*.*\.(cleanup|delete|remove|persist|save|write|sync)\(/) ||
    line.match(/let\s+_\s*=\s*(cleanup|delete|remove|persist|save|write|sync)\(/)
  ) {
    return 'ignored cleanup or persistence result';
  }

  if (line.includes('artifacts_json') && !isAllowedArtifactsJsonPath(relativePath)) {
    return 'legacy artifacts_json outside migration runtime';
  }

  return null;
}

export function checkStorageFallbacks({ rootDir = defaultRootDir } = {}) {
  const errors = [];

  for (const scanDir of SCAN_DIRS) {
    for (const filePath of walkSync(path.join(rootDir, scanDir))) {
      const relativePath = normalizeRelative(rootDir, filePath);
      if (isTestFile(relativePath)) continue;

      const lines = fs.readFileSync(filePath, 'utf-8').split('\n');
      lines.forEach((line, index) => {
        const reason = classifyLine(line, relativePath);
        if (reason) {
          errors.push({
            file: relativePath,
            line: index + 1,
            reason,
          });
        }
      });
    }
  }

  errors.sort((a, b) => a.file.localeCompare(b.file) || a.line - b.line);
  return errors;
}

function printErrors(errors) {
  for (const error of errors) {
    console.error(`[ERROR] ${error.reason} at ${error.file}:${error.line}`);
  }
}

if (process.argv[1] === __filename) {
  const errors = checkStorageFallbacks();
  if (errors.length > 0) {
    printErrors(errors);
    process.exit(1);
  }
}
