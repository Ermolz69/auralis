import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { emptyLibrary, type LibraryAsset, type LibraryState } from '@/shared/api/contracts/library';
import {
  attachAssetToProject,
  copyAsset,
  createOwner,
  createText,
  importFiles,
  importYoutube,
} from './operations';

const { invoke, open } = vi.hoisted(() => ({ invoke: vi.fn(), open: vi.fn() }));
vi.mock('@/shared/api/tauri', () => ({ invoke }));
vi.mock('@tauri-apps/plugin-dialog', () => ({ open }));
const destination = { projectId: null, folderId: null };
const asset = (id = 'source'): LibraryAsset => ({
  id,
  ownerProjectId: 'owner',
  name: 'Source',
  kind: 'text',
  artifactId: null,
  format: 'txt',
  text: 'Accepted text',
  parentId: null,
  tags: ['original'],
  openedAt: null,
});
let state: LibraryState;
let owner: number;
const mutate = vi.fn(async (change: (draft: LibraryState) => void) => {
  const draft = structuredClone(state);
  change(draft);
  state = draft;
});
const refresh = vi.fn(async () => {});
beforeEach(() => {
  state = emptyLibrary();
  owner = 0;
  vi.clearAllMocks();
  invoke.mockReset();
  open.mockReset();
  mutate.mockClear();
  refresh.mockReset().mockResolvedValue(undefined);
  invoke.mockImplementation(async (command: string) => {
    if (command === 'create_project_cmd') return { id: 'owner-' + ++owner };
    if (command === 'delete_project_cmd') return null;
    if (command === 'get_library_cmd') return state;
    throw new Error('Unexpected command: ' + command);
  });
});
afterEach(() => vi.useRealTimers());

describe('library ownership and imports', () => {
  it('creates a durable owner and preserves a distinct text version', async () => {
    const created = await createText('Draft', 'Words', destination, mutate, 'parent');
    expect(state.standaloneOwners).toEqual(['owner-1']);
    expect(state.assets).toEqual([created]);
    expect(created).toMatchObject({
      name: 'Draft',
      text: 'Words',
      parentId: 'parent',
      ownerProjectId: 'owner-1',
    });
  });

  it('rolls back owner creation after persistence fails', async () => {
    const error = new Error('Save failed');
    await expect(createOwner('Draft', vi.fn().mockRejectedValue(error))).rejects.toBe(error);
    expect(invoke).toHaveBeenLastCalledWith('delete_project_cmd', { projectId: 'owner-1' });
    expect(state.assets).toEqual([]);
  });

  it('explains recovery when owner rollback also fails', async () => {
    invoke
      .mockResolvedValueOnce({ id: 'orphan' })
      .mockRejectedValueOnce(new Error('Cannot delete'));
    await expect(
      createOwner('Draft', vi.fn().mockRejectedValue(new Error('Cannot save'))),
    ).rejects.toThrow('доступен в библиотеке для восстановления');
    expect(invoke).toHaveBeenLastCalledWith('delete_project_cmd', { projectId: 'orphan' });
  });

  it('keeps the accepted owner when the subsequent asset write fails', async () => {
    const save = vi
      .fn()
      .mockImplementationOnce(mutate)
      .mockRejectedValueOnce(new Error('Disk full'));
    await expect(createText('Draft', 'Words', destination, save)).rejects.toThrow('Disk full');
    expect(state.standaloneOwners).toEqual(['owner-1']);
    expect(state.assets).toEqual([]);
  });

  it('admits no operations when file picking is cancelled', async () => {
    open.mockResolvedValue(null);
    expect(await importFiles(destination, mutate, refresh)).toEqual([]);
    expect(invoke).not.toHaveBeenCalled();
    expect(mutate).not.toHaveBeenCalled();
    expect(refresh).not.toHaveBeenCalled();
    expect(open).toHaveBeenCalledWith(expect.objectContaining({ multiple: true }));
  });

  it('imports a single TXT picked as a string and refreshes after persistence', async () => {
    open.mockResolvedValue('C:/input/NOTES.TXT');
    const original = invoke.getMockImplementation()!;
    invoke.mockImplementation(async (command, args) =>
      command === 'read_library_text_cmd' ? 'Read words' : original(command, args),
    );
    const ids = await importFiles(destination, mutate, refresh);
    expect(ids).toEqual([state.assets[0].id]);
    expect(state.assets[0]).toMatchObject({ name: 'NOTES.TXT', text: 'Read words' });
    expect(refresh).toHaveBeenCalledOnce();
    expect(mutate.mock.invocationCallOrder.at(-1)).toBeLessThan(
      refresh.mock.invocationCallOrder[0],
    );
  });

  it.each(['srt', 'VTT', 'mp4', 'mkv', 'webm', 'mov', 'wav', 'mp3', 'flac', 'm4a'])(
    'publishes %s only after a ready artifact and refreshed catalog',
    async (extension) => {
      vi.useFakeTimers();
      const original = invoke.getMockImplementation()!;
      let polls = 0;
      invoke.mockImplementation(async (command, args) => {
        if (command === 'list_project_artifacts_cmd')
          return [{ state: ++polls > 1 ? 'ready' : 'pendingFinalize' }];
        if (command === 'import_translation_source_cmd' || command === 'import_local_media_cmd')
          return null;
        return original(command, args);
      });
      refresh.mockImplementation(async () => {
        if (!state.assets.length)
          state.assets.push({ ...asset(), ownerProjectId: 'owner-1', name: 'backend name' });
      });
      const importing = importFiles(destination, mutate, refresh, [
        'C:\\input\\media.' + extension,
      ]);
      await vi.advanceTimersByTimeAsync(151);
      expect(await importing).toEqual(['source']);
      expect(polls).toBe(2);
      expect(state.assets[0].name).toBe('media.' + extension);
      const subtitle = /srt|vtt/i.test(extension);
      expect(invoke).toHaveBeenCalledWith(
        subtitle ? 'import_translation_source_cmd' : 'import_local_media_cmd',
        {
          projectId: 'owner-1',
          path: 'C:\\input\\media.' + extension,
          ...(subtitle ? { format: extension.toLowerCase() } : {}),
        },
      );
      expect(refresh).toHaveBeenCalledTimes(2);
    },
  );

  it('does not return duplicate imported IDs when a conflict replays the mutation', async () => {
    const original = invoke.getMockImplementation()!;
    invoke.mockImplementation(async (command, args) => {
      if (command === 'import_local_media_cmd') return null;
      if (command === 'list_project_artifacts_cmd') return [{ state: 'ready' }];
      return original(command, args);
    });
    refresh.mockImplementation(async () => {
      state.assets = [{ ...asset(), ownerProjectId: 'owner-1' }];
    });
    const replay = async (change: (draft: LibraryState) => void) => {
      change(structuredClone(state));
      await mutate(change);
    };
    expect(await importFiles(destination, replay, refresh, ['clip.mp4'])).toEqual(['source']);
  });

  it.each(['pendingFinalize', 'failed', 'deleting'])(
    'never publishes an import in state %s',
    async (status) => {
      vi.useFakeTimers();
      const original = invoke.getMockImplementation()!;
      invoke.mockImplementation(async (command, args) => {
        if (command === 'import_local_media_cmd') return null;
        if (command === 'list_project_artifacts_cmd') return [{ state: status }];
        return original(command, args);
      });
      const importing = importFiles(destination, mutate, refresh, ['clip.mp4']);
      const rejected = expect(importing).rejects.toThrow('публикация файла ещё не завершена');
      await vi.advanceTimersByTimeAsync(15300);
      await rejected;
      expect(state.assets).toEqual([]);
      expect(refresh).not.toHaveBeenCalled();
    },
  );

  it('propagates import failure without publishing a library result', async () => {
    const original = invoke.getMockImplementation()!;
    invoke.mockImplementation(async (command, args) => {
      if (command === 'import_translation_source_cmd') throw new Error('Invalid subtitles');
      return original(command, args);
    });
    await expect(importFiles(destination, mutate, refresh, ['broken.srt'])).rejects.toThrow(
      'Invalid subtitles',
    );
    expect(state.assets).toEqual([]);
    expect(refresh).not.toHaveBeenCalled();
  });
});

describe('library copies and publication', () => {
  it('returns destination copy IDs after importing media into a project', async () => {
    const original = invoke.getMockImplementation()!;
    invoke.mockImplementation(async (command, args) => {
      if (command === 'import_local_media_cmd' || command === 'attach_library_asset_cmd')
        return null;
      if (command === 'list_project_artifacts_cmd') return [{ state: 'ready' }];
      if (command === 'get_media_operation_cmd')
        return { status: 'ready', assetId: 'destination-copy' };
      return original(command, args);
    });
    refresh.mockImplementation(async () => {
      state.assets = [{ ...asset(), ownerProjectId: 'owner-1' }, asset('destination-copy')];
    });
    expect(
      await importFiles({ projectId: 'destination', folderId: null }, mutate, refresh, [
        'clip.mp4',
      ]),
    ).toEqual(['destination-copy']);
    expect(invoke).toHaveBeenCalledWith(
      'attach_library_asset_cmd',
      expect.objectContaining({ assetId: 'source', projectId: 'destination' }),
    );
  });

  it('keeps media copy failures failed when a source cannot be resolved', async () => {
    invoke.mockRejectedValueOnce(new Error('Managed file is missing. Restore the source first.'));
    await expect(
      copyAsset({ ...asset(), text: null, artifactId: 'missing' }, destination, mutate, refresh),
    ).rejects.toThrow('Restore the source first');
    expect(mutate).not.toHaveBeenCalled();
    expect(state.assets).toEqual([]);
  });

  it('explains cancellation even when the operation has no detailed error', async () => {
    invoke.mockImplementation(async (command) =>
      command === 'get_media_operation_cmd' ? { status: 'cancelled', error: null } : state,
    );
    await expect(attachAssetToProject(asset(), 'destination')).rejects.toThrow(
      'Добавление отменено',
    );
  });
  it('copies text and tag storage independently of the original', async () => {
    const source = asset();
    state.assets = [source];
    await copyAsset(source, destination, mutate, refresh);
    const copied = state.assets[1];
    expect(copied.id).not.toBe(source.id);
    expect(copied.ownerProjectId).not.toBe(source.ownerProjectId);
    expect(copied).toMatchObject({ name: 'Source — копия', text: source.text, tags: ['original'] });
    expect(copied.tags).not.toBe(source.tags);
    copied.tags.push('copy-only');
    expect(source.tags).toEqual(['original']);
    expect(state.assets[0].name).toBe('Source');
  });

  it('rejects copying an unavailable media source before resolving a path', async () => {
    await expect(
      copyAsset({ ...asset(), text: null }, destination, mutate, refresh),
    ).rejects.toThrow('Сначала загрузите исходный медиафайл');
    expect(invoke).not.toHaveBeenCalled();
  });

  it('resolves and reimports media instead of reusing the source asset', async () => {
    const source = { ...asset(), text: null, artifactId: 'original-file' };
    const original = invoke.getMockImplementation()!;
    invoke.mockImplementation(async (command, args) => {
      if (command === 'resolve_artifact_path_cmd') return 'C:/managed/original.wav';
      if (command === 'import_local_media_cmd') return null;
      if (command === 'list_project_artifacts_cmd') return [{ state: 'ready' }];
      return original(command, args);
    });
    refresh.mockImplementation(async () => {
      state.assets = [{ ...asset('copy'), ownerProjectId: 'owner-1' }];
    });
    await copyAsset(source, destination, mutate, refresh);
    expect(invoke).toHaveBeenCalledWith('resolve_artifact_path_cmd', {
      artifactId: 'original-file',
    });
    expect(state.assets[0]).toMatchObject({
      id: 'copy',
      name: 'Source — копия',
      tags: ['original'],
    });
  });

  it.each(['failed', 'cancelled'])(
    'rejects %s attachment even when a partial asset exists',
    async (status) => {
      state.assets = [asset('partial')];
      const original = invoke.getMockImplementation()!;
      invoke.mockImplementation(async (command, args) => {
        if (command === 'attach_library_asset_cmd') return state;
        if (command === 'get_media_operation_cmd')
          return { status, assetId: 'partial', error: 'Copy stopped' };
        return original(command, args);
      });
      await expect(attachAssetToProject(asset(), 'destination')).rejects.toThrow('Copy stopped');
      expect(invoke).toHaveBeenCalledWith(
        'attach_library_asset_cmd',
        expect.objectContaining({
          projectId: 'destination',
          assetId: 'source',
          duplicate: false,
        }),
      );
    },
  );

  it('waits for publication rather than trusting a ready operation without its asset', async () => {
    vi.useFakeTimers();
    const copied = asset('copy');
    let polls = 0;
    const original = invoke.getMockImplementation()!;
    invoke.mockImplementation(async (command, args) => {
      if (command === 'attach_library_asset_cmd') return state;
      if (command === 'get_library_cmd') {
        if (++polls === 2) state.assets.push(copied);
        return state;
      }
      if (command === 'get_media_operation_cmd') return { status: 'ready', assetId: 'copy' };
      return original(command, args);
    });
    const copying = attachAssetToProject(asset(), 'destination');
    await vi.advanceTimersByTimeAsync(301);
    expect(await copying).toBe(copied);
    expect(polls).toBe(2);
  });

  it('recognizes an already published link when no operation is retained', async () => {
    state.links = [{ projectId: 'destination', assetId: 'source', folderId: null }];
    invoke.mockImplementation(async (command) =>
      command === 'get_media_operation_cmd' ? null : state,
    );
    const source = asset();
    expect(await attachAssetToProject(source, 'destination')).toBe(source);
  });

  it('times out pending copies with a recovery message, never a partial result', async () => {
    vi.useFakeTimers();
    invoke.mockImplementation(async (command) =>
      command === 'get_media_operation_cmd' ? { status: 'copying', assetId: 'partial' } : state,
    );
    const copying = attachAssetToProject(asset(), 'destination');
    const rejected = expect(copying).rejects.toThrow('появится в проекте автоматически');
    await vi.advanceTimersByTimeAsync(120300);
    await rejected;
  });

  it('registers a YouTube owner before refresh and attaches catalog assets to the destination', async () => {
    const original = invoke.getMockImplementation()!;
    invoke.mockImplementation(async (command, args) => {
      if (command === 'create_project_from_youtube_cmd') return { id: 'youtube' };
      if (command === 'attach_library_asset_cmd') return state;
      if (command === 'get_media_operation_cmd') return { status: 'ready', assetId: 'copy' };
      return original(command, args);
    });
    refresh.mockImplementation(async () => {
      expect(state.standaloneOwners).toContain('youtube');
      state.assets = [{ ...asset(), ownerProjectId: 'youtube' }, asset('copy'), asset('foreign')];
    });
    await importYoutube(
      'https://youtube.com/watch?v=fixture',
      { ...destination, projectId: 'project' },
      mutate,
      refresh,
    );
    expect(refresh).toHaveBeenCalledTimes(2);
    expect(
      invoke.mock.calls.filter(([command]) => command === 'attach_library_asset_cmd'),
    ).toHaveLength(1);
  });
});
