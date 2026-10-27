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
export async function attachAssetToProject(
  asset: LibraryAsset,
  projectId: string,
): Promise<LibraryAsset> {
  const operationId = crypto.randomUUID();
  await invoke('attach_library_asset_cmd', {
    operationId,
    projectId,
    assetId: asset.id,
    duplicate: false,
  });
  const deadline = Date.now() + 120000;
  while (Date.now() < deadline) {
    const state = await invoke('get_library_cmd');
    const op = await invoke('get_media_operation_cmd', { operationId });
    if (
      !op &&
      state.links.some((link) => link.projectId === projectId && link.assetId === asset.id)
    )
      return asset;
    if (op?.status === 'ready') {
      const copy = state.assets.find((a) => a.id === op.assetId);
      if (copy) return copy;
    }
    if (op && ['failed', 'cancelled'].includes(op.status))
      throw new Error(op.error ?? 'Добавление отменено');
    await new Promise((resolve) => setTimeout(resolve, 300));
  }
  throw new Error('Публикация материала продолжается. Он появится в проекте автоматически.');
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
  });
  return destination.projectId ? attachAssetToProject(asset, destination.projectId) : asset;
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
          'Импорт сохранён, но публикация файла ещё не завершена. Материал появится в библиотеке автоматически.',
        );
      await new Promise((resolve) => setTimeout(resolve, 150));
    }
    await refresh();
    await mutate((state) => {
      for (const asset of state.assets.filter((a) => a.ownerProjectId === owner)) {
        asset.name = name;

        imported.push(asset.id);
      }
    });
    if (destination.projectId) {
      const catalog = await invoke('get_library_cmd');
      for (const original of catalog.assets.filter((a) => a.ownerProjectId === owner)) {
        const copied = await attachAssetToProject(original, destination.projectId);
        const index = imported.indexOf(original.id);
        if (index >= 0) imported[index] = copied.id;
      }
    }
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
  if (destination.projectId) {
    const state = await invoke('get_library_cmd');
    for (const asset of state.assets.filter((a) => a.ownerProjectId === project.id))
      await attachAssetToProject(asset, destination.projectId);
    await refresh();
  }
}
