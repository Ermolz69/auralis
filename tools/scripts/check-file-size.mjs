import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const defaultRootDir = path.resolve(__dirname, '../../');

const RULES = [
  { dir: 'apps/desktop/src/pages', maxLines: 120, pattern: /\.(ts|tsx)$/ },
  { dir: 'apps/desktop/src/widgets', maxLines: 250, pattern: /\.(ts|tsx)$/ },
  { dir: 'apps/desktop/src/features', maxLines: 250, pattern: /\.(ts|tsx)$/ },
  { dir: 'apps/desktop/src/entities', maxLines: 300, pattern: /\.(ts|tsx)$/ },
  { dir: 'apps/desktop/src/shared/ui', maxLines: 200, pattern: /\.(ts|tsx)$/ },
  { dir: 'apps/desktop/src/shared/lib', maxLines: 250, pattern: /\.(ts|tsx)$/ },
  { dir: 'crates/application/src', maxLines: 300, pattern: /\.rs$/ },
];

// Existing oversized modules are ratcheted at their current size. This keeps the
// quality gate useful: they cannot grow further while they are split incrementally.
const FILE_LIMIT_OVERRIDES = new Map([
  ['apps/desktop/src/features/project-list/ui/ProjectList.tsx', 276],
  ['apps/desktop/src/pages/project/ui/ProjectPage.stories.tsx', 162],
  ['crates/application/src/usecases/transcript/import_youtube_subtitles/usecase.rs', 304],
]);

const EXCLUDED_SUFFIXES = [
  'pnpm-lock.yaml',
  'Cargo.lock',
  '.generated.ts',
  '.d.ts',
  '.snap',
  '.svg',
];
const EXCLUDED_SEGMENTS = ['node_modules', 'dist', 'target', '__generated__', 'api-types'];
const STATIC_SUFFIXES = ['data.ts', 'constants.ts'];
const TEST_PATTERNS = [
  /\.test\.(ts|tsx)$/,
  /\.spec\.(ts|tsx)$/,
  /(^|\/)tests\.rs$/,
  /(^|\/)tests\//,
  /(^|\/)__tests__\//,
];

function normalizeRelative(rootDir, filePath) {
  return path.relative(rootDir, filePath).replace(/\\/g, '/');
}

function getRuleForFile(rootDir, filePath) {
  const relativePath = normalizeRelative(rootDir, filePath);
  const override = FILE_LIMIT_OVERRIDES.get(relativePath);
  if (override !== undefined) return { maxLines: override };

  for (const rule of RULES) {
    const fullDirPath = path.join(rootDir, path.normalize(rule.dir));
    if (filePath.startsWith(fullDirPath) && rule.pattern.test(filePath)) {
      return rule;
    }
  }

  if (filePath.includes(`${path.sep}adapters-`) && filePath.endsWith('.rs')) {
    return { maxLines: 400 };
  }

  return null;
}

function isExcluded(rootDir, filePath) {
  const relativePath = normalizeRelative(rootDir, filePath);
  const segments = relativePath.split('/');

  if (EXCLUDED_SEGMENTS.some((segment) => segments.includes(segment))) return true;
  if (EXCLUDED_SUFFIXES.some((suffix) => relativePath.endsWith(suffix))) return true;
  if (STATIC_SUFFIXES.some((suffix) => relativePath.endsWith(suffix))) return true;
  if (TEST_PATTERNS.some((pattern) => pattern.test(relativePath))) return true;

  return false;
}

function walkSync(dir, filelist = []) {
  if (!fs.existsSync(dir)) return filelist;

  for (const file of fs.readdirSync(dir).sort()) {
    const filepath = path.join(dir, file);
    if (fs.statSync(filepath).isDirectory()) {
      walkSync(filepath, filelist);
    } else {
      filelist.push(filepath);
    }
  }

  return filelist;
}

export function checkFileSize({ rootDir = defaultRootDir } = {}) {
  const errors = [];
  const searchRoots = [path.join(rootDir, 'apps/desktop/src'), path.join(rootDir, 'crates')];

  for (const searchRoot of searchRoots) {
    for (const file of walkSync(searchRoot)) {
      if (isExcluded(rootDir, file)) continue;

      const rule = getRuleForFile(rootDir, file);
      if (!rule) continue;

      const content = fs.readFileSync(file, 'utf-8');
      const lines = countLinesForPolicy(file, content);

      if (lines > rule.maxLines) {
        errors.push({
          file: normalizeRelative(rootDir, file),
          lines,
          maxLines: rule.maxLines,
        });
      }
    }
  }

  errors.sort((a, b) => a.file.localeCompare(b.file));
  return errors;
}

function countLinesForPolicy(filePath, content) {
  if (!filePath.endsWith('.rs')) {
    return content.split('\n').length;
  }

  return stripRustCfgTestModules(content).split('\n').length;
}

function stripRustCfgTestModules(content) {
  const lines = content.split('\n');
  const kept = [];
  let pendingCfgTest = false;
  let skipping = false;
  let braceDepth = 0;

  for (const line of lines) {
    const trimmed = line.trim();

    if (!skipping && trimmed === '#[cfg(test)]') {
      pendingCfgTest = true;
      continue;
    }

    if (pendingCfgTest && /^mod\s+tests\s*\{/.test(trimmed)) {
      skipping = true;
      pendingCfgTest = false;
      braceDepth = countBraces(line);
      if (braceDepth <= 0) skipping = false;
      continue;
    }

    if (skipping) {
      braceDepth += countBraces(line);
      if (braceDepth <= 0) skipping = false;
      continue;
    }

    if (pendingCfgTest) {
      kept.push('#[cfg(test)]');
      pendingCfgTest = false;
    }

    kept.push(line);
  }

  return kept.join('\n');
}

function countBraces(line) {
  return (line.match(/\{/g) ?? []).length - (line.match(/\}/g) ?? []).length;
}

function printErrors(errors) {
  for (const error of errors) {
    console.error(`ERROR: File too large [${error.lines}/${error.maxLines}] ${error.file}`);
  }
}

if (process.argv[1] === __filename) {
  const errors = checkFileSize();
  if (errors.length > 0) {
    printErrors(errors);
    process.exit(1);
  }

  console.log('SUCCESS: All file sizes are within architectural limits.');
}
