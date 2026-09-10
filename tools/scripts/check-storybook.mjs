import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const ALLOWED_ROOTS = ['Design System/', 'Product/'];
const ALLOWED_CONTROL_TYPES = new Set(['boolean', 'range', 'radio', 'select', 'text']);

export function collectStorybookErrors({
  stories,
  mainSource,
  managerSource,
  introductionSource,
  previewSource,
}) {
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
  const globalControls = extractObjectProperty(previewSource, 'controls');
  if (!globalControls || !/\bdisable\s*:\s*true\b/.test(globalControls)) {
    errors.push('.storybook/preview.tsx: controls must be disabled by default');
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
    validateControls(file, metaSource, source.slice(metaEnd), errors);
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
    previewSource: fs.readFileSync(path.join(desktopDir, '.storybook/preview.tsx'), 'utf8'),
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

function validateControls(file, metaSource, storySource, errors) {
  if (/\bcontrols\s*:\s*\{[\s\S]*?\bdisable\s*:\s*false\b/.test(storySource)) {
    errors.push(`${file}: story-level controls cannot bypass the meta safety policy`);
  }

  const controlsSource = extractObjectProperty(metaSource, 'controls');
  if (!controlsSource) return;

  const disabled = controlsSource.match(/\bdisable\s*:\s*(true|false)\b/)?.[1];
  if (!disabled) {
    errors.push(`${file}: controls must declare disable: true or disable: false`);
    return;
  }

  const include = parseStringArrayProperty(controlsSource, 'include');
  if (disabled === 'true') {
    if (include) errors.push(`${file}: disabled controls cannot declare an include list`);
    return;
  }

  if (!include?.length) {
    errors.push(`${file}: enabled controls require a non-empty literal include list`);
    return;
  }

  const argTypesSource = extractObjectProperty(metaSource, 'argTypes');
  if (!argTypesSource) {
    errors.push(`${file}: enabled controls require explicit argTypes`);
    return;
  }

  const argTypeKeys = collectTopLevelKeys(argTypesSource);
  for (const controlName of include) {
    if (!argTypeKeys.includes(controlName)) {
      errors.push(`${file}: included control "${controlName}" is missing from argTypes`);
      continue;
    }

    const definition = extractObjectProperty(argTypesSource, controlName) ?? '';
    const directType = definition.match(/\bcontrol\s*:\s*['"]([^'"]+)['"]/)?.[1];
    const controlObject = extractObjectProperty(definition, 'control');
    const structuredType = controlObject?.match(/\btype\s*:\s*['"]([^'"]+)['"]/)?.[1];
    const controlType = directType ?? structuredType;

    if (!controlType || !ALLOWED_CONTROL_TYPES.has(controlType)) {
      errors.push(
        `${file}: control "${controlName}" must use boolean, range, radio, select, or text`,
      );
      continue;
    }
    if (
      (controlType === 'select' || controlType === 'radio') &&
      !/\boptions\s*:/.test(definition)
    ) {
      errors.push(`${file}: control "${controlName}" requires finite options`);
    }
    if (
      controlType === 'range' &&
      (!controlObject ||
        !/\bmin\s*:\s*-?\d/.test(controlObject) ||
        !/\bmax\s*:\s*-?\d/.test(controlObject) ||
        !/\bstep\s*:\s*\d/.test(controlObject))
    ) {
      errors.push(`${file}: range control "${controlName}" requires numeric min, max, and step`);
    }
  }

  for (const argTypeKey of argTypeKeys) {
    if (!include.includes(argTypeKey)) {
      errors.push(`${file}: argType "${argTypeKey}" must be listed in controls.include`);
    }
  }
}

function extractObjectProperty(source, property) {
  const match = new RegExp(`\\b${property}\\s*:\\s*\\{`).exec(source);
  if (!match) return null;

  const openingBrace = match.index + match[0].lastIndexOf('{');
  let depth = 0;
  let quote = null;
  let escaped = false;

  for (let index = openingBrace; index < source.length; index += 1) {
    const character = source[index];
    if (quote) {
      if (escaped) escaped = false;
      else if (character === '\\') escaped = true;
      else if (character === quote) quote = null;
      continue;
    }
    if (character === "'" || character === '"' || character === '`') {
      quote = character;
      continue;
    }
    if (character === '{') depth += 1;
    if (character === '}') {
      depth -= 1;
      if (depth === 0) return source.slice(openingBrace + 1, index);
    }
  }

  return null;
}

function parseStringArrayProperty(source, property) {
  const match = new RegExp(`\\b${property}\\s*:\\s*\\[([^\\]]*)\\]`).exec(source);
  if (!match) return null;
  const values = [...match[1].matchAll(/['"]([^'"]+)['"]/g)].map((entry) => entry[1]);
  return values.length > 0 ? values : null;
}

function collectTopLevelKeys(objectSource) {
  const candidates = [...objectSource.matchAll(/^([ \t]+)([A-Za-z_$][\w$]*)\s*:/gm)];
  if (candidates.length === 0) return [];
  const minimumIndent = Math.min(...candidates.map((match) => match[1].length));
  return candidates.filter((match) => match[1].length === minimumIndent).map((match) => match[2]);
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
