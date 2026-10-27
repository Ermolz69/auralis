import { expect, it } from 'vitest';
import type { LibraryTab } from '@/shared/api/contracts/library';
import { workspacePanels } from './panels';
it('preserves one shared speech controller across preparation, synthesis and voices', () => {
  const tabs: LibraryTab[] = [
    { id: 'text', tool: 'text', assetId: 'a', draft: '{}' },
    { id: 'speech', tool: 'speech', assetId: 'a', draft: '{}' },
    { id: 'voices', tool: 'voices', assetId: 'a', draft: '{}' },
    { id: 'other', tool: 'text', assetId: 'b', draft: '{}' },
  ];
  const panels = workspacePanels(tabs, 'speech');
  expect(panels).toHaveLength(2);
  expect(panels[0]).toMatchObject({ key: 'speech:a', active: true, tab: { id: 'speech' } });
  expect(panels[1].active).toBe(false);
  expect(workspacePanels(tabs, 'voices')[0].key).toBe(panels[0].key);
});
