import assert from 'node:assert/strict';
import test from 'node:test';
import { collectStorybookErrors } from './check-storybook.mjs';

const validStory = `
const meta = {
  title: 'Design System/Components/Button',
  parameters: {
    docs: { description: { component: 'Primary action primitive.' } },
  },
  tags: ['autodocs'],
} satisfies Meta;
export const Primary = {};
`;

function input(stories = [['Button.stories.tsx', validStory]]) {
  return {
    stories,
    mainSource: "stories: ['../src/**/*.mdx']; disableTelemetry: true;",
    managerSource: "brandTitle: 'Auralis Design System'",
    previewSource: 'parameters: { controls: { disable: true } }',
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
