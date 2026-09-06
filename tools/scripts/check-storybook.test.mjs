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
    introductionSource: '# Introduction',
  });

  assert.ok(errors.some((error) => error.includes('MDX documentation glob is missing')));
  assert.ok(errors.some((error) => error.includes('telemetry must stay disabled')));
  assert.ok(errors.some((error) => error.includes('manager branding is missing')));
  assert.ok(errors.some((error) => error.includes('Design System introduction is missing')));
});
