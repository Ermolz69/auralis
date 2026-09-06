import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const ALLOWED_ROOTS = ['Design System/', 'Product/'];

export function collectStorybookErrors({ stories, mainSource, managerSource, introductionSource }) {
  const errors = [];
  const titles = new Map();

  if (!/\.mdx['"]/.test(mainSource)) {
    errors.push('.storybook/main.ts: MDX documentation glob is missing');
  }
  if (!mainSource.includes('disableTelemetry: true')) {
    errors.push('.storybook/main.ts: Storybook telemetry must stay disabled');
  }
  if (!managerSource.includes("brandTitle: 'Auralis Design System'")) {
    errors.push('.storybook/auralisTheme.ts: Auralis manager branding is missing');
  }
  if (!introductionSource.includes('<Meta title="Design System/Introduction" />')) {
    errors.push('Introduction.mdx: canonical Design System introduction is missing');
  }

  for (const [file, source] of stories) {
    const metaStart = source.search(/const\s+meta\s*=\s*\{/);
    const metaEnd = source.indexOf('satisfies Meta', metaStart);
    if (metaStart === -1 || metaEnd === -1) {
      errors.push(`${file}: typed CSF meta declaration is missing`);
      continue;
    }

    const metaSource = source.slice(metaStart, metaEnd);
    const title = metaSource.match(/\btitle:\s*'([^']+)'/)?.[1];
    if (!title) {
      errors.push(`${file}: meta.title is missing or is not a stable string`);
    } else {
      if (!ALLOWED_ROOTS.some((root) => title.startsWith(root))) {
        errors.push(`${file}: title must start with ${ALLOWED_ROOTS.join(' or ')}`);
      }
      const firstFile = titles.get(title);
      if (firstFile) {
        errors.push(`${file}: duplicate title "${title}" already used by ${firstFile}`);
      } else {
        titles.set(title, file);
      }
    }

    if (!/tags:\s*\[[^\]]*['"]autodocs['"][^\]]*\]/s.test(metaSource)) {
      errors.push(`${file}: meta must enable the autodocs tag`);
    }
    if (!/docs\s*:\s*\{[\s\S]*?description\s*:\s*\{[\s\S]*?component\s*:/s.test(metaSource)) {
      errors.push(`${file}: meta.parameters.docs.description.component is required`);
    }
    if (![...source.matchAll(/^export\s+const\s+\w+/gm)].length) {
      errors.push(`${file}: at least one named story export is required`);
    }
  }

  return errors;
}

export function verifyStorybook(rootDir) {
  const desktopDir = path.join(rootDir, 'apps/desktop');
  const srcDir = path.join(desktopDir, 'src');
  const storyFiles = findFiles(srcDir, (file) => file.endsWith('.stories.tsx'));
  const stories = storyFiles.map((file) => [
    path.relative(rootDir, file).replaceAll('\\', '/'),
    fs.readFileSync(file, 'utf8'),
  ]);
  const errors = collectStorybookErrors({
    stories,
    mainSource: fs.readFileSync(path.join(desktopDir, '.storybook/main.ts'), 'utf8'),
    managerSource: fs.readFileSync(path.join(desktopDir, '.storybook/auralisTheme.ts'), 'utf8'),
    introductionSource: fs.readFileSync(
      path.join(srcDir, 'shared/ui/design-tokens/Introduction.mdx'),
      'utf8',
    ),
  });

  if (errors.length > 0) {
    throw new Error(`Invalid Storybook design-system contract:\n- ${errors.join('\n- ')}`);
  }

  return { storyFiles: stories.length, titles: stories.length };
}

function findFiles(directory, predicate) {
  const files = [];
  for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
    const absolutePath = path.join(directory, entry.name);
    if (entry.isDirectory()) files.push(...findFiles(absolutePath, predicate));
    else if (predicate(absolutePath)) files.push(absolutePath);
  }
  return files.sort();
}

const currentFile = fileURLToPath(import.meta.url);
if (path.resolve(process.argv[1] ?? '') === currentFile) {
  const rootDir = path.resolve(path.dirname(currentFile), '../..');
  try {
    const result = verifyStorybook(rootDir);
    process.stdout.write(
      `Storybook design-system contract is valid (${result.storyFiles} documented story files).\n`,
    );
  } catch (error) {
    process.stderr.write(`${error.message}\n`);
    process.exitCode = 1;
  }
}
