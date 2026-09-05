import assert from 'node:assert/strict';
import test from 'node:test';
import { ESLint } from 'eslint';

const eslint = new ESLint({ cwd: import.meta.dirname });

async function lint(source, filePath) {
  const [result] = await eslint.lintText(source, { filePath });
  assert.equal(result.fatalErrorCount, 0);
  assert.equal(result.warningCount, 0);
  return result.messages.map(({ ruleId }) => ruleId);
}

test('allows imports from features into public entity APIs', async () => {
  assert.deepEqual(
    await lint("import '@/entities/project';", 'src/features/import-local-media/ui/check.ts'),
    [],
  );
});

test('rejects upward imports from shared into features', async () => {
  assert.deepEqual(
    await lint("import '@/features/import-local-media';", 'src/shared/api/check.ts'),
    ['boundaries/dependencies'],
  );
});

test('rejects deep imports that bypass an entity public API', async () => {
  assert.ok(
    (
      await lint(
        "import '@/entities/project/model/types';",
        'src/features/import-local-media/ui/check.ts',
      )
    ).includes('no-restricted-imports'),
  );
});

test('allows relative imports within a slice', async () => {
  assert.deepEqual(
    await lint("import './ImportLocalMediaButton';", 'src/features/import-local-media/ui/check.ts'),
    [],
  );
});

test('rejects direct Lucide imports outside the shared icon registry', async () => {
  assert.ok(
    (
      await lint("import { Info } from 'lucide-react';", 'src/widgets/media-panel/ui/check.tsx')
    ).includes('no-restricted-imports'),
  );
});

test('keeps Storybook recommended rules enabled', async () => {
  assert.ok(
    (await lint('export const Primary = {};', 'src/shared/ui/button/check.stories.tsx')).includes(
      'storybook/default-exports',
    ),
  );
});
