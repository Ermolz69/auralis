import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const currentFile = fileURLToPath(import.meta.url);
const defaultRootDir = path.resolve(path.dirname(currentFile), '../..');
const minimumLines = 12;
const excluded = [
  /\.test\.(ts|tsx)$/,
  /\.stories\.(ts|tsx)$/,
  /\.story(Data|Fixtures)\.(ts|tsx)$/,
  /\.d\.ts$/,
  /(^|\/)index\.ts$/,
  // The central allow-list intentionally repeats imported identifiers as object shorthand.
  /(^|\/)shared\/ui\/icon\/registry\.ts$/,
];

function walk(directory, files = []) {
  if (!fs.existsSync(directory)) return files;
  for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
    const target = path.join(directory, entry.name);
    if (entry.isDirectory()) walk(target, files);
    else files.push(target);
  }
  return files;
}

function normalizedLines(content) {
  return content.split(/\r?\n/).flatMap((source, lineIndex) => {
    const text = source.trim().replace(/\s+/g, ' ');
    if (!text || text.startsWith('//') || text.startsWith('import ') || text.startsWith('export *'))
      return [];
    return [{ text, sourceLine: lineIndex + 1 }];
  });
}

function relative(rootDir, file) {
  return path.relative(rootDir, file).replaceAll('\\', '/');
}

export function findDuplicateCode({ rootDir = defaultRootDir } = {}) {
  const sourceRoot = path.join(rootDir, 'apps/desktop/src');
  const windows = new Map();

  for (const file of walk(sourceRoot).sort()) {
    const relativePath = relative(rootDir, file);
    if (!/\.(ts|tsx)$/.test(file) || excluded.some((pattern) => pattern.test(relativePath)))
      continue;
    const lines = normalizedLines(fs.readFileSync(file, 'utf8'));
    for (let index = 0; index <= lines.length - minimumLines; index += 1) {
      const block = lines.slice(index, index + minimumLines);
      const key = block.map((line) => line.text).join('\n');
      const occurrence = {
        file: relativePath,
        normalizedLine: index,
        sourceLine: block[0].sourceLine,
      };
      const existing = windows.get(key);
      if (existing) existing.push(occurrence);
      else windows.set(key, [occurrence]);
    }
  }

  const duplicates = [];
  for (const occurrences of windows.values()) {
    for (let left = 0; left < occurrences.length; left += 1) {
      for (let right = left + 1; right < occurrences.length; right += 1) {
        const first = occurrences[left];
        const second = occurrences[right];
        if (
          first.file === second.file &&
          Math.abs(first.normalizedLine - second.normalizedLine) < minimumLines
        )
          continue;
        duplicates.push({ first, second });
      }
    }
  }

  return duplicates
    .sort((left, right) =>
      `${left.first.file}:${left.first.sourceLine}:${left.second.file}:${left.second.sourceLine}`.localeCompare(
        `${right.first.file}:${right.first.sourceLine}:${right.second.file}:${right.second.sourceLine}`,
      ),
    )
    .filter((duplicate, index, all) => {
      const previous = all[index - 1];
      return !(
        previous &&
        duplicate.first.file === previous.first.file &&
        duplicate.second.file === previous.second.file &&
        duplicate.first.normalizedLine === previous.first.normalizedLine + 1 &&
        duplicate.second.normalizedLine === previous.second.normalizedLine + 1
      );
    })
    .map(({ first, second }) => ({
      first: { file: first.file, line: first.sourceLine },
      second: { file: second.file, line: second.sourceLine },
      lines: minimumLines,
    }));
}

if (process.argv[1] === currentFile) {
  const duplicates = findDuplicateCode();
  for (const duplicate of duplicates) {
    console.error(
      `ERROR: Duplicate production block (${duplicate.lines}+ meaningful lines): ` +
        `${duplicate.first.file}:${duplicate.first.line} and ${duplicate.second.file}:${duplicate.second.line}`,
    );
  }
  if (duplicates.length > 0) process.exit(1);
  console.log('SUCCESS: No large exact duplicates found in frontend production code.');
}
