import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import test from 'node:test';
import { collectStorybookErrors, findStoryFiles, inspectStorybook } from './check-storybook.mjs';
import { inside, loadStorybookTypeScript } from './check-storybook-typescript.mjs';

const validStory = `
const meta = {
  title: 'Design System/Components/Button',
  component: Button,
  parameters: {
    docs: { description: { component: 'Primary action primitive.' } },
  },
  tags: ['autodocs'],
} satisfies Meta;
export default meta;
export const Primary = {};
`;

function input(stories = [['Button.stories.tsx', validStory]]) {
  return {
    stories,
    mainSource:
      "export default { stories: ['../src/**/*.mdx'], core: { disableTelemetry: true } };",
    managerSource: "brandTitle: 'Auralis Design System'",
    previewSource:
      "const preview = { parameters: { controls: { disable: true }, a11y: { test: 'error' } } }; export default preview;",
    introductionSource: '<Meta title="Design System/Introduction" />',
  };
}

test('accepts a documented and uniquely titled story catalog', () => {
  assert.deepEqual(collectStorybookErrors(input()), []);
});

test('rejects undocumented story metadata', () => {
  const story = validStory
    .replace("tags: ['autodocs'],", '')
    .replace("docs: { description: { component: 'Primary action primitive.' } },", '');
  const errors = collectStorybookErrors(input([['Button.stories.tsx', story]]));

  assert.ok(errors.includes('Button.stories.tsx: meta must enable the autodocs tag'));
  assert.ok(
    errors.includes('Button.stories.tsx: meta.parameters.docs.description.component is required'),
  );
});

test('rejects legacy roots and duplicate catalog paths', () => {
  const legacyStory = validStory.replace('Design System/Components/Button', 'Shared UI/Button');
  const errors = collectStorybookErrors(
    input([
      ['First.stories.tsx', legacyStory],
      ['Second.stories.tsx', legacyStory],
    ]),
  );

  assert.ok(errors.includes('First.stories.tsx: title must start with Design System/ or Product/'));
  assert.ok(errors.some((error) => error.includes('duplicate title "Shared UI/Button"')));
});

test('rejects an incomplete Storybook configuration', () => {
  const errors = collectStorybookErrors({
    ...input(),
    mainSource: 'export default {};',
    managerSource: 'export const theme = {};',
    previewSource: 'export default {};',
    introductionSource: '# Introduction',
  });

  assert.ok(errors.some((error) => error.includes('MDX documentation glob is missing')));
  assert.ok(errors.some((error) => error.includes('telemetry must stay disabled')));
  assert.ok(errors.some((error) => error.includes('manager branding is missing')));
  assert.ok(errors.some((error) => error.includes('controls must be disabled by default')));
  assert.ok(errors.some((error) => error.includes('Design System introduction is missing')));
});

test('accepts an explicit allowlist of bounded primitive controls', () => {
  const story = validStory
    .replace(
      'parameters: {',
      `parameters: {
    controls: {
      disable: false,
      include: ['variant', 'disabled', 'progress'],
    },`,
    )
    .replace(
      "tags: ['autodocs'],",
      `tags: ['autodocs'],
  argTypes: {
    variant: { control: 'select', options: ['primary', 'secondary'] },
    disabled: { control: 'boolean' },
    progress: { control: { type: 'range', min: 0, max: 100, step: 1 } },
  },`,
    );

  assert.deepEqual(collectStorybookErrors(input([['Button.stories.tsx', story]])), []);
});

test('rejects enabled controls without a complete allowlist and argTypes contract', () => {
  const story = validStory
    .replace(
      'parameters: {',
      `parameters: {
    controls: { disable: false, include: ['state', 'missing'] },`,
    )
    .replace(
      "tags: ['autodocs'],",
      `tags: ['autodocs'],
  argTypes: {
    state: { control: 'object' },
  },`,
    );
  const errors = collectStorybookErrors(input([['Unsafe.stories.tsx', story]]));

  assert.ok(errors.some((error) => error.includes('control "state" must use')));
  assert.ok(errors.some((error) => error.includes('control "missing" is missing from argTypes')));
});

test('rejects unbounded select and range controls', () => {
  const story = validStory
    .replace(
      'parameters: {',
      `parameters: {
    controls: { disable: false, include: ['variant', 'progress'] },`,
    )
    .replace(
      "tags: ['autodocs'],",
      `tags: ['autodocs'],
  argTypes: {
    variant: { control: 'select' },
    progress: { control: { type: 'range', min: 0, max: 100 } },
  },`,
    );
  const errors = collectStorybookErrors(input([['Unbounded.stories.tsx', story]]));

  assert.ok(errors.some((error) => error.includes('control "variant" requires finite options')));
  assert.ok(errors.some((error) => error.includes('range control "progress" requires')));
});

test('rejects story-level attempts to re-enable controls', () => {
  const story = `${validStory}\nexport const Unsafe = { parameters: { controls: { disable: false } } };`;
  const errors = collectStorybookErrors(input([['Override.stories.tsx', story]]));

  assert.ok(errors.some((error) => error.includes('story-level controls cannot bypass')));
});

test('requires a nonempty catalogue and an actual typed default export', () => {
  assert.ok(collectStorybookErrors(input([])).some((error) => error.includes('empty')));
  const source = validStory.replace('export default meta;', '');
  assert.ok(collectStorybookErrors(input([['Missing.stories.tsx', source]])).length > 0);
});

test('comments and quoted lookalikes do not supply metadata or unsafe overrides', () => {
  const source =
    validStory + `\n// controls: { disable: false }\nconst text = "a11y: { disable: true }";`;
  assert.deepEqual(collectStorybookErrors(input([['Safe.stories.tsx', source]])), []);
  const fake = `/* ${validStory} */\nexport const Placeholder = {};`;
  assert.ok(collectStorybookErrors(input([['Fake.stories.tsx', fake]])).length > 0);
});

for (const [label, parameters] of [
  ['disabled accessibility', '{ a11y: { disable: true } }'],
  ['nonblocking accessibility', "{ a11y: { test: 'todo' } }"],
  ['manual accessibility', '{ a11y: { manual: true } }'],
  ['disabled rule', "{ a11y: { config: { rules: [{ id: 'color-contrast', enabled: false }] } } }"],
  [
    'disabled run option rule',
    "{ a11y: { options: { rules: { 'color-contrast': { enabled: false } } } } }",
  ],
  ['narrow run options', "{ a11y: { options: { runOnly: ['color-contrast'] } } }"],
  ['narrow configuration', "{ a11y: { config: { runOnly: ['color-contrast'] } } }"],
  ['opaque parameters', 'externalParameters'],
]) {
  test(`rejects ${label} in a story override`, () => {
    const source = validStory.replace(
      'export const Primary = {};',
      `export const Primary = { parameters: ${parameters} };`,
    );
    assert.ok(collectStorybookErrors(input([['Unsafe.stories.tsx', source]])).length > 0);
  });
}

test('requires error-level global accessibility and refuses hidden test stories', () => {
  const preview = input();
  preview.previewSource = preview.previewSource.replace("test: 'error'", "test: 'off'");
  assert.ok(collectStorybookErrors(preview).some((error) => error.includes('a11y')));
  const source = validStory.replace(
    'export const Primary = {};',
    "export const Primary = { tags: ['!test'] };",
  );
  assert.ok(collectStorybookErrors(input([['Hidden.stories.tsx', source]])).length > 0);
});

test('resolves local parameter inheritance instead of overlooking a disabled rule', () => {
  const source = `const unsafe = { a11y: { disable: true } };\n${validStory}`.replace(
    'export const Primary = {};',
    'export const Primary = { parameters: { ...unsafe } };',
  );
  assert.ok(collectStorybookErrors(input([['Inherited.stories.tsx', source]])).length > 0);
});

for (const [label, definition] of [
  ['empty options', "{ control: 'select', options: [] }"],
  ['opaque options', "{ control: 'radio', options: getOptions() }"],
  ['reversed range', "{ control: { type: 'range', min: 100, max: 0, step: 1 } }"],
  ['zero range step', "{ control: { type: 'range', min: 0, max: 100, step: 0 } }"],
  ['nonfinite numeric expression', "{ control: { type: 'range', min: 0, max: 1 / 0, step: 1 } }"],
]) {
  test(`rejects ${label}`, () => {
    const source = validStory
      .replace('parameters: {', "parameters: { controls: { disable: false, include: ['value'] },")
      .replace("tags: ['autodocs'],", `tags: ['autodocs'], argTypes: { value: ${definition} },`);
    assert.ok(collectStorybookErrors(input([['Bounds.stories.tsx', source]])).length > 0);
  });
}

test('does not confuse presentational rendering or ordinary argument names with test opt-outs', () => {
  const source = validStory.replace(
    'export const Primary = {};',
    'export const Primary = { args: { a11y: { disable: true }, test: { skip: true } } };',
  );
  assert.deepEqual(collectStorybookErrors(input([['Presentational.stories.tsx', source]])), []);
});

function fixture(t) {
  const parent = path.resolve(
    fileURLToPath(new URL('../../', import.meta.url)),
    '.cache/storybook-contract/fixtures',
  );
  fs.mkdirSync(parent, { recursive: true });
  const directory = fs.mkdtempSync(path.join(parent, 'case-'));
  t.after(() => {
    assert.ok(directory !== parent && inside(parent, directory));
    fs.rmSync(directory, { recursive: true, force: true });
  });
  return directory;
}

test('TypeScript cannot fall back to an ancestor package or execute it before rejecting', (t) => {
  const directory = fixture(t),
    invoked = path.join(directory, 'owned');
  fs.mkdirSync(path.join(invoked, 'apps/desktop'), { recursive: true });
  fs.writeFileSync(path.join(invoked, 'apps/desktop/package.json'), '{}');
  const dependency = path.join(directory, 'node_modules/typescript');
  fs.mkdirSync(dependency, { recursive: true });
  fs.writeFileSync(path.join(dependency, 'package.json'), '{"main":"index.js"}');
  const sentinel = path.join(directory, 'executed');
  fs.writeFileSync(
    path.join(dependency, 'index.js'),
    `require('fs').writeFileSync(${JSON.stringify(sentinel)}, 'executed');`,
  );
  assert.throws(() => loadStorybookTypeScript(invoked), /inside the owned repository/);
  assert.equal(fs.existsSync(sentinel), false);
});

test('a repository-local TypeScript symlink cannot escape the realpath boundary', (t) => {
  const directory = fixture(t),
    invoked = path.join(directory, 'owned');
  fs.mkdirSync(path.join(invoked, 'apps/desktop'), { recursive: true });
  fs.writeFileSync(path.join(invoked, 'apps/desktop/package.json'), '{}');
  const dependency = path.join(directory, 'external-typescript');
  fs.mkdirSync(dependency);
  fs.writeFileSync(path.join(dependency, 'package.json'), '{"main":"index.js"}');
  const sentinel = path.join(directory, 'executed');
  fs.writeFileSync(
    path.join(dependency, 'index.js'),
    `require('fs').writeFileSync(${JSON.stringify(sentinel)}, 'executed');`,
  );
  fs.mkdirSync(path.join(invoked, 'apps/desktop/node_modules'));
  fs.symlinkSync(
    dependency,
    path.join(invoked, 'apps/desktop/node_modules/typescript'),
    process.platform === 'win32' ? 'junction' : 'dir',
  );
  assert.throws(() => loadStorybookTypeScript(invoked), /inside the owned repository/);
  assert.equal(fs.existsSync(sentinel), false);
});

test('discovers all registered CSF extensions rather than only TSX files', (t) => {
  const directory = fixture(t);
  for (const extension of ['js', 'jsx', 'mjs', 'ts', 'tsx'])
    fs.writeFileSync(path.join(directory, `Example.stories.${extension}`), '');
  fs.writeFileSync(path.join(directory, 'NotAStory.tsx'), '');
  assert.equal(findStoryFiles(directory).length, 5);
});

test('reports custom, inherited and render-only stories without inventing behavior coverage', () => {
  const source = validStory
    .replace("tags: ['autodocs'],", "tags: ['autodocs'], play: async () => {},")
    .replace(
      'export const Primary = {};',
      'export const Primary = {}; export const Custom = { play: async () => {} };',
    );
  const inherited = inspectStorybook(input([['Inherited.stories.tsx', source]]));
  assert.deepEqual(inherited.errors, []);
  assert.deepEqual(inherited.metrics, {
    storyFiles: 1,
    stories: 2,
    customPlay: 1,
    inheritedPlay: 1,
    renderOnly: 0,
  });
  const passive = inspectStorybook(input());
  assert.equal(passive.metrics.renderOnly, 1);
  assert.equal(passive.metrics.customPlay, 0);
});

test('local args spreads, pure parameter helpers and finite registry constants remain valid', () => {
  const source =
    `const icons = { First: null, Second: null }; const args = { children: 'Safe' };\nfunction parameters() { return { layout: 'padded' }; }\n${validStory}`
      .replace('parameters: {', "parameters: { controls: { disable: false, include: ['name'] },")
      .replace(
        "tags: ['autodocs'],",
        "tags: ['autodocs'], argTypes: { name: { control: 'select', options: Object.keys(icons) } },",
      )
      .replace(
        'export const Primary = {};',
        'export const Primary = { args: { ...args }, parameters: parameters() };',
      );
  assert.deepEqual(collectStorybookErrors(input([['Finite.stories.tsx', source]])), []);
});

test('imported safety parameters are inspected as source without executing the helper module', (t) => {
  const directory = fixture(t),
    helper = path.join(directory, 'parameters.ts');
  const sentinel = path.join(directory, 'executed');
  fs.writeFileSync(
    helper,
    `import fs from 'node:fs'; fs.writeFileSync(${JSON.stringify(sentinel)}, 'executed'); export const unsafe = { a11y: { disable: true } };`,
  );
  const source = `import { unsafe } from './parameters';\n${validStory}`.replace(
    'export const Primary = {};',
    'export const Primary = { parameters: unsafe };',
  );
  assert.ok(
    collectStorybookErrors(input([[path.join(directory, 'Imported.stories.tsx'), source]])).some(
      (error) => error.includes('a11y.disable'),
    ),
  );
  assert.equal(fs.existsSync(sentinel), false);
});

test('include-only controls and story argTypes cannot redefine the meta control policy', () => {
  for (const replacement of [
    "export const Primary = { parameters: { controls: { include: ['data'] } } };",
    "export const Primary = { argTypes: { data: { control: 'object' } } };",
  ]) {
    const source = validStory.replace('export const Primary = {};', replacement);
    assert.ok(
      collectStorybookErrors(input([['Override.stories.tsx', source]])).some((error) =>
        error.includes('story-level controls'),
      ),
    );
  }
});

test('an unresolved accessibility flag is rejected instead of assumed false', () => {
  const source = validStory.replace(
    'export const Primary = {};',
    'export const Primary = { parameters: { a11y: { disable: importedFlag } } };',
  );
  assert.ok(collectStorybookErrors(input([['Opaque.stories.tsx', source]])).length > 0);
});

test('global and meta test opt-outs cannot hide the catalog', () => {
  const global = input();
  global.previewSource = global.previewSource.replace(
    'parameters:',
    "tags: ['!test'], parameters:",
  );
  assert.ok(collectStorybookErrors(global).some((error) => error.includes('test gate')));
  const source = validStory.replace("tags: ['autodocs'],", "tags: ['autodocs', '!test'],");
  assert.ok(
    collectStorybookErrors(input([['Hidden.stories.tsx', source]])).some((error) =>
      error.includes('test gate'),
    ),
  );
});

test('imported finite registries and pure metadata helper arguments stay statically resolvable', (t) => {
  const directory = fixture(t);
  fs.writeFileSync(
    path.join(directory, 'metadata.ts'),
    'export const icons = { First: null, Second: null }; export function parameters(label) { return { layout: label }; }',
  );
  const source = `import { icons, parameters } from './metadata';\n${validStory}`
    .replace('parameters: {', "parameters: { controls: { disable: false, include: ['name'] },")
    .replace(
      "tags: ['autodocs'],",
      "tags: ['autodocs'], argTypes: { name: { control: 'select', options: Object.keys(icons) } },",
    )
    .replace(
      'export const Primary = {};',
      "export const Primary = { parameters: parameters('padded') };",
    );
  assert.deepEqual(
    collectStorybookErrors(input([[path.join(directory, 'Imported.stories.tsx'), source]])),
    [],
  );
});
