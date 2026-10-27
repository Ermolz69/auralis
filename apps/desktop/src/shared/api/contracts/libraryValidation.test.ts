import { expect, it } from 'vitest';
import { emptyLibrary } from './library';
import { validateLibrary } from './libraryValidation';
it('rejects malformed library responses before rendering', () => {
  expect(validateLibrary(emptyLibrary())).toBe(true);
  expect(validateLibrary({ ...emptyLibrary(), revision: -1 })).toBe(false);
  expect(
    validateLibrary({
      ...emptyLibrary(),
      workspaces: [
        {
          id: 'x',
          projectId: null,
          assetId: 'a',
          activeTab: null,
          tabs: [{ id: 't', tool: 'invalid', assetId: 'a', draft: '{}' }],
        },
      ],
    }),
  ).toBe(false);
  expect(validateLibrary({ ...emptyLibrary(), browser: null })).toBe(false);
  expect(validateLibrary({ ...emptyLibrary(), links: [{ assetId: 'a', projectId: 'p' }] })).toBe(
    false,
  );
});
