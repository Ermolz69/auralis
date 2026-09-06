import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const REQUIRED_DOCS = [
  'README.md',
  'apps/desktop/README.md',
  'docs/README.md',
  'docs/architecture/000-stack.md',
  'docs/architecture/001-overview.md',
  'docs/architecture/002-frontend-fsd.md',
  'docs/architecture/003-rust-workspace.md',
  'docs/architecture/004-design-system.md',
  'docs/architecture/005-storybook-conventions.md',
  'docs/ci/001-quality-gates.md',
  'docs/taskfile/001-commands.md',
];

export function collectDocumentationErrors({ markdownFiles, requiredDocs = REQUIRED_DOCS }) {
  const normalizedFiles = new Map(
    [...markdownFiles].map(([file, source]) => [normalizePath(file), source]),
  );
  const errors = [];

  for (const requiredDoc of requiredDocs) {
    if (!normalizedFiles.has(normalizePath(requiredDoc))) {
      errors.push(`missing mandatory documentation file: ${requiredDoc}`);
    }
  }

  for (const [file, source] of normalizedFiles) {
    for (const match of source.matchAll(/!?\[[^\]]*\]\(([^)]+)\)/g)) {
      const rawTarget = match[1].trim();
      if (/^(?:https?:|mailto:|#)/i.test(rawTarget)) continue;

      const targetWithoutTitle =
        rawTarget.match(/^<([^>]+)>/)?.[1] ?? rawTarget.split(/\s+["']/)[0];
      let targetPath;
      try {
        targetPath = decodeURIComponent(targetWithoutTitle.split('#')[0]);
      } catch {
        errors.push(`${file}: invalid encoded Markdown link: ${targetWithoutTitle}`);
        continue;
      }
      if (!targetPath) continue;

      const resolved = normalizePath(path.posix.join(path.posix.dirname(file), targetPath));
      if (!normalizedFiles.has(resolved)) {
        errors.push(`${file}: broken relative Markdown link: ${targetWithoutTitle}`);
      }
    }
  }

  return errors;
}

export function verifyDocumentation(rootDir) {
  const markdownFiles = findMarkdownFiles(rootDir).map((file) => [
    normalizePath(path.relative(rootDir, file)),
    fs.readFileSync(file, 'utf8'),
  ]);
  const errors = collectDocumentationErrors({ markdownFiles });

  if (errors.length > 0) {
    throw new Error(`Invalid project documentation:\n- ${errors.join('\n- ')}`);
  }

  return { files: markdownFiles.length, required: REQUIRED_DOCS.length };
}

function findMarkdownFiles(rootDir) {
  const roots = [path.join(rootDir, 'README.md'), path.join(rootDir, 'apps/desktop/README.md')];
  const docsDir = path.join(rootDir, 'docs');
  const pending = [docsDir];

  while (pending.length > 0) {
    const directory = pending.pop();
    for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
      const absolutePath = path.join(directory, entry.name);
      if (entry.isDirectory()) pending.push(absolutePath);
      else if (entry.name.endsWith('.md')) roots.push(absolutePath);
    }
  }

  return roots.sort();
}

function normalizePath(file) {
  return path.posix.normalize(file.replaceAll('\\', '/'));
}

const currentFile = fileURLToPath(import.meta.url);
if (path.resolve(process.argv[1] ?? '') === currentFile) {
  const rootDir = path.resolve(path.dirname(currentFile), '../..');
  try {
    const result = verifyDocumentation(rootDir);
    process.stdout.write(
      `Documentation contract is valid (${result.files} Markdown files, ${result.required} mandatory).\n`,
    );
  } catch (error) {
    process.stderr.write(`${error.message}\n`);
    process.exitCode = 1;
  }
}
