import { expect, it } from 'vitest';
import { emptyLibrary } from './library';
import { validateLibrary } from './libraryValidation';
import { IpcContractError, parseCommandResult } from './runtimeValidation';

function populatedLibrary() {
  const library = emptyLibrary();
  library.revision = 7;
  library.assets = [
    {
      id: 'source',
      ownerProjectId: 'project',
      name: 'Original source',
      kind: 'text',
      artifactId: null,
      format: 'txt',
      text: 'Unchanged original',
      parentId: null,
      tags: ['Russian'],
      openedAt: null,
    },
  ];
  library.folders = [{ id: 'folder', projectId: 'project', parentId: null, name: 'Sources' }];
  library.links = [{ assetId: 'source', projectId: 'project', folderId: 'folder' }];
  library.workspaces = [
    {
      id: 'workspace',
      projectId: 'project',
      assetId: null,
      activeTab: 'speech-tab',
      tabs: [{ id: 'speech-tab', tool: 'speech', assetId: 'source', draft: '{"text":"Draft"}' }],
    },
  ];
  library.browser = {
    scope: 'project',
    projectId: 'project',
    folderId: 'folder',
    search: 'source',
    kind: 'text',
    selectedId: 'source',
  };
  return library;
}

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

it('preserves the selected material, folder, workspace and draft in a populated IPC catalog', () => {
  const library = populatedLibrary();
  expect(parseCommandResult('get_library_cmd', library)).toBe(library);
  expect(parseCommandResult('save_library_cmd', library)).toBe(library);
  expect(library.workspaces[0].tabs[0].draft).toBe('{"text":"Draft"}');
  expect(library.browser.selectedId).toBe(library.links[0].assetId);
});

it.each(['material-array', 'tool-array', 'material-object'])(
  'rejects %s instead of coercing a damaged enum at the IPC boundary',
  (damage) => {
    const library = populatedLibrary();
    const damaged = JSON.parse(JSON.stringify(library));
    if (damage === 'tool-array') damaged.workspaces[0].tabs[0].tool = ['speech'];
    else damaged.assets[0].kind = damage === 'material-array' ? ['text'] : { toString: null };
    expect(() => parseCommandResult('get_library_cmd', damaged)).toThrow(IpcContractError);
  },
);

it('accepts checked material lineage and rejects missing context identity', () => {
  const context = {
    projectId: 'project',
    assetId: 'source',
    version: 'source-v2',
    artifactId: 'ready',
    trackId: null,
  };
  expect(parseCommandResult('get_tool_context_cmd', context)).toBe(context);
  expect(() => parseCommandResult('get_tool_context_cmd', { ...context, assetId: null })).toThrow(
    IpcContractError,
  );
});

it('distinguishes absent copy work from bounded progress and rejects damaged counters', () => {
  const operation = {
    id: 'copy',
    projectId: 'project',
    assetId: 'source',
    status: 'Copying',
    completedBytes: 3,
    totalBytes: 9,
    error: null,
  };
  expect(parseCommandResult('get_media_operation_cmd', null)).toBeNull();
  expect(parseCommandResult('get_media_operation_cmd', operation)).toBe(operation);
  expect(() =>
    parseCommandResult('get_media_operation_cmd', { ...operation, completedBytes: -1 }),
  ).toThrow(IpcContractError);
  expect(parseCommandResult('cancel_media_operation_cmd', null)).toBeNull();
  expect(parseCommandResult('validate_media_project_cmd', null)).toBeNull();
});

it('rejects an incompatible media-project protocol before publishing its material list', () => {
  const project = { version: 1, project_id: 'project', assets: [] };
  expect(parseCommandResult('get_media_project_cmd', project)).toBe(project);
  expect(() => parseCommandResult('get_media_project_cmd', { ...project, version: 2 })).toThrow(
    IpcContractError,
  );
});
