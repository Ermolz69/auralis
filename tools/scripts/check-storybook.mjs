import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { loadStorybookTypeScript } from './check-storybook-typescript.mjs';
import { sourceModule, defaultValue, exportedStories } from './check-storybook-source.mjs';
import { accessibility, controls, storyPolicy, tags } from './check-storybook-policies.mjs';

const ownRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
const storyExtension = /\.stories\.(?:js|jsx|mjs|ts|tsx)$/;

export function inspectStorybook({
  stories,
  mainSource,
  managerSource,
  introductionSource,
  previewSource,
  rootDir = ownRoot,
}) {
  const { ts, root, path: parserPath } = loadStorybookTypeScript(rootDir);
  const errors = [],
    titles = new Map();
  const metrics = {
    storyFiles: stories.length,
    stories: 0,
    customPlay: 0,
    inheritedPlay: 0,
    renderOnly: 0,
  };
  if (!stories.length) errors.push('Storybook catalogue is empty');
  const module = (source, file) => sourceModule(ts, source, file, root);
  try {
    const main = defaultValue(module(mainSource, '.storybook/main.ts')).object();
    if (
      !main
        .get('stories')
        ?.strings()
        .some((glob) => glob.endsWith('.mdx'))
    )
      errors.push('.storybook/main.ts: MDX documentation glob is missing');
    if (main.get('core')?.object().get('disableTelemetry')?.scalar() !== true)
      errors.push('.storybook/main.ts: Storybook telemetry must stay disabled');
  } catch {
    errors.push(
      '.storybook/main.ts: MDX documentation glob is missing; telemetry must stay disabled',
    );
  }
  if (!managerSource.includes("brandTitle: 'Auralis Design System'"))
    errors.push('.storybook/auralisTheme.ts: Auralis manager branding is missing');
  if (!introductionSource.includes('<Meta title="Design System/Introduction" />'))
    errors.push('Introduction.mdx: canonical Design System introduction is missing');
  try {
    const preview = defaultValue(module(previewSource, '.storybook/preview.tsx')).object();
    tags(preview.get('tags'));
    const parameters = preview.get('parameters')?.object();
    if (parameters?.get('controls')?.object().get('disable')?.scalar() !== true)
      errors.push('.storybook/preview.tsx: controls must be disabled by default');
    if (parameters?.get('a11y')?.object().get('test')?.scalar() !== 'error')
      errors.push('.storybook/preview.tsx: a11y.test must remain error');
    accessibility(parameters ?? new Map());
  } catch (error) {
    errors.push(
      `.storybook/preview.tsx: controls must be disabled by default; a11y policy: ${error.message}`,
    );
  }
  for (const [file, source] of stories) {
    try {
      const parsed = module(source, file),
        exported = defaultValue(parsed);
      let typed = exported.node;
      const aliases = new Set();
      while (ts.isIdentifier(typed) && parsed.declarations.has(typed.text)) {
        if (aliases.has(typed.text)) throw new Error('Cyclic Storybook meta export');
        aliases.add(typed.text);
        typed = parsed.declarations.get(typed.text);
      }
      if (
        !ts.isSatisfiesExpression(typed) ||
        !ts.isTypeReferenceNode(typed.type) ||
        typed.type.typeName.getText() !== 'Meta'
      )
        throw new Error('typed CSF meta declaration is missing');
      const meta = exported.object(),
        title = meta.get('title')?.scalar();
      if (typeof title !== 'string' || !title)
        errors.push(`${file}: meta.title is missing or is not a stable string`);
      else {
        if (!['Design System/', 'Product/'].some((prefix) => title.startsWith(prefix)))
          errors.push(`${file}: title must start with Design System/ or Product/`);
        if (titles.has(title))
          errors.push(`${file}: duplicate title "${title}" already used by ${titles.get(title)}`);
        else titles.set(title, file);
      }
      if (!tags(meta.get('tags')).includes('autodocs'))
        errors.push(`${file}: meta must enable the autodocs tag`);
      const parameters = meta.get('parameters')?.object() ?? new Map();
      const description = parameters
        .get('docs')
        ?.object()
        .get('description')
        ?.object()
        .get('component')
        ?.scalar();
      if (typeof description !== 'string' || !description.trim())
        errors.push(`${file}: meta.parameters.docs.description.component is required`);
      accessibility(parameters);
      for (const message of controls(parameters, meta.get('argTypes')?.object()))
        errors.push(`${file}: ${message}`);
      const entries = exportedStories(parsed, meta);
      if (!entries.length) errors.push(`${file}: at least one named story export is required`);
      for (const entry of entries) {
        const story = entry.value.object();
        storyPolicy(meta, story);
        const play = story.get('play') ?? meta.get('play');
        if (play && !play.callable())
          throw new Error(`${entry.name}: play must be a resolved function`);
        metrics.stories++;
        if (story.has('play')) metrics.customPlay++;
        else if (meta.has('play')) metrics.inheritedPlay++;
        else metrics.renderOnly++;
      }
    } catch (error) {
      errors.push(`${file}: ${error.message}`);
    }
  }
  return {
    errors,
    metrics,
    parser: { version: ts.version, path: path.relative(root, parserPath).replaceAll('\\', '/') },
  };
}

export function collectStorybookErrors(input) {
  return inspectStorybook(input).errors;
}

export function verifyStorybook(rootDir) {
  const desktop = path.join(rootDir, 'apps/desktop');
  const stories = findStoryFiles(path.join(desktop, 'src')).map((file) => [
    path.relative(rootDir, file).replaceAll('\\', '/'),
    fs.readFileSync(file, 'utf8'),
  ]);
  const result = inspectStorybook({
    rootDir,
    stories,
    mainSource: fs.readFileSync(path.join(desktop, '.storybook/main.ts'), 'utf8'),
    managerSource: fs.readFileSync(path.join(desktop, '.storybook/auralisTheme.ts'), 'utf8'),
    previewSource: fs.readFileSync(path.join(desktop, '.storybook/preview.tsx'), 'utf8'),
    introductionSource: fs.readFileSync(
      path.join(desktop, 'src/shared/ui/design-tokens/Introduction.mdx'),
      'utf8',
    ),
  });
  if (result.errors.length)
    throw new Error(`Invalid Storybook design-system contract:\n- ${result.errors.join('\n- ')}`);
  return result;
}

export function findStoryFiles(directory) {
  return fs
    .readdirSync(directory, { withFileTypes: true })
    .flatMap((entry) => {
      const file = path.join(directory, entry.name);
      return entry.isDirectory() ? findStoryFiles(file) : storyExtension.test(file) ? [file] : [];
    })
    .sort();
}

if (path.resolve(process.argv[1] ?? '') === fileURLToPath(import.meta.url)) {
  try {
    const { metrics, parser } = verifyStorybook(ownRoot);
    process.stdout.write(
      `Storybook contract valid: ${metrics.storyFiles} files, ${metrics.stories} stories; ${metrics.customPlay} custom play, ${metrics.inheritedPlay} inherited play, ${metrics.renderOnly} render-only. Play presence is not an assertion-quality measure.\n`,
    );
    process.stdout.write(`Owned TypeScript ${parser.version}: ${parser.path}\n`);
  } catch (error) {
    process.stderr.write(`${error.message}\n`);
    process.exitCode = 1;
  }
}
