import { describe, expect, it } from 'vitest';
import { emptyLibrary, type LibraryAsset } from '@/shared/api/contracts/library';
import { newWorkspace, openTab, visibleAssets } from './catalog';
const asset = (id: string): LibraryAsset => ({
  id,
  ownerProjectId: 'owner',
  name: id,
  kind: 'text',
  artifactId: null,
  format: 'txt',
  text: '',
  parentId: null,
  tags: [],
  openedAt: null,
});
describe('library navigation', () => {
  it('keeps project workspaces separate from standalone assets and reuses a tool tab', () => {
    const project = newWorkspace('p', null);
    const standalone = newWorkspace(null, 'a');
    expect(project.id).not.toBe(standalone.id);
    openTab(project, 'text', 'a');
    const id = project.activeTab;
    openTab(project, 'speech', 'a');
    openTab(project, 'text', 'a');
    expect(project.tabs).toHaveLength(2);
    expect(project.activeTab).toBe(id);
    expect(standalone.tabs).toEqual([]);
  });
  it('filters shared memberships, nested folders, tags and hidden assets', () => {
    const state = emptyLibrary();
    state.assets = [asset('a'), asset('b'), asset('c')];
    state.assets[0].tags = ['Русский'];
    state.folders = [
      { id: 'root', projectId: 'p', parentId: null, name: 'Root' },
      { id: 'child', projectId: 'p', parentId: 'root', name: 'Child' },
    ];
    state.links = [
      { assetId: 'a', projectId: 'p', folderId: 'child' },
      { assetId: 'a', projectId: 'q', folderId: null },
    ];
    expect(
      visibleAssets(state, {
        ...state.browser,
        scope: 'project',
        projectId: 'p',
        folderId: 'root',
        search: 'русский',
      }).map((a) => a.id),
    ).toEqual(['a']);
    state.hiddenAssets = ['c'];
    expect(
      visibleAssets(state, { ...state.browser, scope: 'unassigned' }).map((a) => a.id),
    ).toEqual(['b']);
    state.links = state.links.filter((l) => l.projectId !== 'p');
    expect(
      visibleAssets(state, { ...state.browser, scope: 'project', projectId: 'q' }).map((a) => a.id),
    ).toEqual(['a']);
  });
});
