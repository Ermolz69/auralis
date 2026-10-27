import type { LibraryTab } from '@/shared/api/contracts/library';
export const panelKey = (tab: LibraryTab) =>
  ['text', 'speech', 'voices'].includes(tab.tool) ? `speech:${tab.assetId ?? 'project'}` : tab.id;
export function workspacePanels(tabs: LibraryTab[], activeId: string | null) {
  const groups = new Map<string, LibraryTab[]>();
  for (const tab of tabs) {
    const key = panelKey(tab);
    groups.set(key, [...(groups.get(key) ?? []), tab]);
  }
  return [...groups].map(([key, group]) => ({
    key,
    tab: group.find((t) => t.id === activeId) ?? group[0],
    active: group.some((t) => t.id === activeId),
  }));
}
