import { open } from '@tauri-apps/plugin-dialog';
import { invoke } from '@/shared/api/tauri';
import type { LibraryAsset, LibraryState } from '@/shared/api/contracts/library';
type Mutate = (change: (draft: LibraryState) => void) => Promise<void>;
export type Destination = { projectId: string | null; folderId: string | null };
export async function createOwner(name: string, mutate: Mutate) {
  const owner = await invoke('create_project_cmd', { title: name });
  try {
    await mutate((draft) => {
      draft.standaloneOwners.push(owner.id);
    });
  } catch (reason) {
    try {
      await invoke('delete_project_cmd', { projectId: owner.id });
    } catch {
      throw new Error(
        'Не удалось сохранить контекст материала. Созданный пустой проект доступен в библиотеке для восстановления.',
      );
    }
    throw reason;
  }
  return owner.id;
}
export function linkAsset(state: LibraryState, assetId: string, destination: Destination) {
  if (!destination.projectId) return;
  const link = state.links.find(
    (l) => l.assetId === assetId && l.projectId === destination.projectId,
  );
  if (link) link.folderId = destination.folderId;
  else
    state.links.push({ assetId, projectId: destination.projectId, folderId: destination.folderId });
}
export async function createText(
  name: string,
  text: string,
  destination: Destination,
  mutate: Mutate,
  parentId: string | null = null,
): Promise<LibraryAsset> {
  const ownerProjectId = await createOwner(name, mutate);
  const asset: LibraryAsset = {
    id: crypto.randomUUID(),
    ownerProjectId,
    name,
    kind: 'text',
    text,
    artifactId: null,
    format: 'txt',
    parentId,
    tags: [],
    openedAt: null,
  };
  await mutate((state) => {
    state.assets.push(asset);
    linkAsset(state, asset.id, destination);
  });
  return asset;
}
export async function importFiles(
  destination: Destination,
  mutate: Mutate,
  refresh: () => Promise<void>,
  paths?: string[],
) {
  const imported: string[] = [];
  const picked =
    paths ??
    (await open({
      multiple: true,
      filters: [
        {
          name: 'Материалы Auralis',
          extensions: [
            'txt',
            'srt',
            'vtt',
            'mp4',
            'mkv',
            'webm',
            'mov',
            'wav',
            'mp3',
            'flac',
            'm4a',
          ],
        },
      ],
    }));
  if (!picked) return imported;
  for (const path of typeof picked === 'string' ? [picked] : picked) {
    const name = path.split(/[\\/]/).pop() || 'Материал';
    if (/\.txt$/i.test(path)) {
      const asset = await createText(
        name,
        await invoke('read_library_text_cmd', { path }),
        destination,
        mutate,
      );
      imported.push(asset.id);
      continue;
    }
    const owner = await createOwner(name, mutate);
    if (/\.(srt|vtt)$/i.test(path))
      await invoke('import_translation_source_cmd', {
        projectId: owner,
        path,
        format: /\.vtt$/i.test(path) ? 'vtt' : 'srt',
      });
    else await invoke('import_local_media_cmd', { projectId: owner, path });
    const deadline = Date.now() + 15000;
    while (
      !(await invoke('list_project_artifacts_cmd', { projectId: owner })).some(
        (a) => a.state === 'ready',
      )
    ) {
      if (Date.now() > deadline)
        throw new Error(
          'Импорт сохранён, но публикация файла ещё не завершена. Обновите библиотеку и проверьте результаты.',
        );
      await new Promise((resolve) => setTimeout(resolve, 150));
    }
    await refresh();
    await mutate((state) => {
      for (const asset of state.assets.filter((a) => a.ownerProjectId === owner)) {
        asset.name = name;
        linkAsset(state, asset.id, destination);
        imported.push(asset.id);
      }
    });
  }
  await refresh();
  return imported;
}
export async function copyAsset(
  asset: LibraryAsset,
  destination: Destination,
  mutate: Mutate,
  refresh: () => Promise<void>,
) {
  let copied: string[];
  if (asset.text !== null) {
    copied = [(await createText(`${asset.name} — копия`, asset.text, destination, mutate)).id];
  } else {
    if (!asset.artifactId)
      throw new Error('Сначала загрузите исходный медиафайл, затем создайте копию.');
    const path = await invoke('resolve_artifact_path_cmd', { artifactId: asset.artifactId });
    copied = await importFiles(destination, mutate, refresh, [path]);
  }
  await mutate((state) => {
    for (const item of state.assets.filter((a) => copied.includes(a.id))) {
      item.name = `${asset.name} — копия`;
      item.tags = [...asset.tags];
    }
    if (destination.projectId && copied.length)
      state.links = state.links.filter(
        (l) => l.assetId !== asset.id || l.projectId !== destination.projectId,
      );
  });
}
export async function importYoutube(
  url: string,
  destination: Destination,
  mutate: Mutate,
  refresh: () => Promise<void>,
) {
  const project = await invoke('create_project_from_youtube_cmd', { url });
  await mutate((state) => {
    state.standaloneOwners.push(project.id);
  });
  await refresh();
  await mutate((state) => {
    for (const asset of state.assets.filter((a) => a.ownerProjectId === project.id))
      linkAsset(state, asset.id, destination);
  });
}
