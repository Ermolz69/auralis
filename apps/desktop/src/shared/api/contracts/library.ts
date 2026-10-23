export type LibraryKind = 'text' | 'subtitle' | 'audio' | 'video';
export type LibraryTool = 'source' | 'text' | 'translate' | 'speech' | 'voices' | 'results';
export type LibraryAsset = {
  id: string;
  ownerProjectId: string;
  name: string;
  kind: LibraryKind;
  artifactId: string | null;
  format: string | null;
  text: string | null;
  parentId: string | null;
  tags: string[];
  openedAt: string | null;
};
export type LibraryFolder = {
  id: string;
  projectId: string;
  parentId: string | null;
  name: string;
};
export type LibraryLink = { assetId: string; projectId: string; folderId: string | null };
export type LibraryTab = { id: string; tool: LibraryTool; assetId: string | null; draft: string };
export type LibraryWorkspace = {
  id: string;
  projectId: string | null;
  assetId: string | null;
  activeTab: string | null;
  tabs: LibraryTab[];
};
export type LibraryBrowser = {
  scope: string;
  projectId: string | null;
  folderId: string | null;
  search: string;
  kind: string;
  selectedId: string | null;
};
export type LibraryState = {
  revision: number;
  assets: LibraryAsset[];
  folders: LibraryFolder[];
  links: LibraryLink[];
  standaloneOwners: string[];
  removedProjects: string[];
  hiddenAssets: string[];
  workspaces: LibraryWorkspace[];
  browser: LibraryBrowser;
};
export interface LibraryCommandMap {
  read_library_text_cmd: { args: { path: string }; result: string };
  get_library_cmd: { args: undefined; result: LibraryState };
  save_library_cmd: { args: { state: LibraryState }; result: LibraryState };
}
export const emptyLibrary = (): LibraryState => ({
  revision: 0,
  assets: [],
  folders: [],
  links: [],
  standaloneOwners: [],
  removedProjects: [],
  hiddenAssets: [],
  workspaces: [],
  browser: {
    scope: 'all',
    projectId: null,
    folderId: null,
    search: '',
    kind: '',
    selectedId: null,
  },
});
