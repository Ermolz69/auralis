import type {
  LibraryAsset,
  LibraryBrowser,
  LibraryState,
  LibraryTool,
  LibraryWorkspace,
} from '@/shared/api/contracts/library';
export const toolNames: Record<LibraryTool, string> = {
  source: 'Источник и субтитры',
  text: 'Текст и подготовка',
  translate: 'Перевод',
  speech: 'Синтез речи',
  voices: 'Голоса',
  results: 'Результаты',
};
export const kindNames = { text: 'Текст', subtitle: 'Субтитры', audio: 'Аудио', video: 'Видео' };
export function visibleAssets(state: LibraryState, browser: LibraryBrowser): LibraryAsset[] {
  const folderIds = new Set(browser.folderId ? [browser.folderId] : []);
  let changed = true;
  while (changed) {
    changed = false;
    for (const folder of state.folders)
      if (folder.parentId && folderIds.has(folder.parentId) && !folderIds.has(folder.id)) {
        folderIds.add(folder.id);
        changed = true;
      }
  }
  const words = browser.search.toLocaleLowerCase('ru').split(/\s+/).filter(Boolean);
  return state.assets.filter(
    (a) =>
      !state.hiddenAssets.includes(a.id) &&
      (!browser.kind || a.kind === browser.kind) &&
      words.every((word) =>
        `${a.name} ${a.tags.join(' ')}`.toLocaleLowerCase('ru').includes(word),
      ) &&
      (browser.scope !== 'unassigned' ||
        !state.links.some(
          (l) => l.assetId === a.id && !state.removedProjects.includes(l.projectId),
        )) &&
      (browser.scope !== 'recent' || a.openedAt) &&
      (browser.scope !== 'project' ||
        state.links.some(
          (l) =>
            l.assetId === a.id &&
            l.projectId === browser.projectId &&
            (!browser.folderId || (l.folderId && folderIds.has(l.folderId))),
        )),
  );
}
export function workspaceKey(projectId: string | null, assetId: string | null) {
  return projectId ? `project:${projectId}` : `asset:${assetId}`;
}
export function newWorkspace(projectId: string | null, assetId: string | null): LibraryWorkspace {
  return { id: workspaceKey(projectId, assetId), projectId, assetId, activeTab: null, tabs: [] };
}
export function openTab(workspace: LibraryWorkspace, tool: LibraryTool, assetId: string | null) {
  let tab = workspace.tabs.find((t) => t.tool === tool && t.assetId === assetId);
  if (!tab) {
    tab = { id: crypto.randomUUID(), tool, assetId, draft: '{}' };
    workspace.tabs.push(tab);
  }
  workspace.activeTab = tab.id;
}
export const defaultTool = (asset: LibraryAsset): LibraryTool =>
  asset.kind === 'video' ? 'source' : asset.kind === 'audio' ? 'results' : 'text';
