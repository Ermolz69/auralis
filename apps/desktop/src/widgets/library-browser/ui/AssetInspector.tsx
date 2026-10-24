import { describeCommandError } from '@/shared/api/contracts/error';
import { useState } from 'react';
import { useLibrary, kindNames, toolNames, useStudioActions } from '@/entities/library';
import { copyAsset } from '@/entities/library';
import type { LibraryAsset, LibraryBrowser, LibraryTool } from '@/shared/api/contracts/library';
import { button, field } from './controls';
import { AssetVersions } from './AssetVersions';
type Props = { asset: LibraryAsset; browser: LibraryBrowser; select: (id: string | null) => void };
export function AssetInspector({ asset, browser, select }: Props) {
  const { state, projects, mutate, refresh, openAsset } = useLibrary();
  const [name, setName] = useState(asset.name);
  const [tags, setTags] = useState(asset.tags.join(', '));
  const [target, setTarget] = useState(browser.projectId ?? '');
  const actions = useStudioActions();
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  const [confirmDelete, setConfirmDelete] = useState(false);
  const links = state.links.filter(
    (l) => l.assetId === asset.id && !state.removedProjects.includes(l.projectId),
  );
  const action = async (work: () => Promise<void>) => {
    setBusy(true);
    setError('');
    try {
      await work();
    } catch (reason) {
      setError(describeCommandError(reason));
    } finally {
      setBusy(false);
    }
  };
  const save = () =>
    action(() =>
      mutate((draft) => {
        const item = draft.assets.find((a) => a.id === asset.id)!;
        item.name = name.trim();
        item.tags = [
          ...new Set(
            tags
              .split(',')
              .map((t) => t.trim())
              .filter(Boolean),
          ),
        ];
      }),
    );
  const attach = () => actions.drop(asset.id, target);
  const remove = () =>
    action(async () => {
      await mutate((draft) => {
        draft.hiddenAssets.push(asset.id);
        draft.links = draft.links.filter((l) => l.assetId !== asset.id);
        draft.workspaces = draft.workspaces.filter((w) => w.assetId !== asset.id);
        for (const workspace of draft.workspaces) {
          workspace.tabs = workspace.tabs.filter((t) => t.assetId !== asset.id);
          workspace.activeTab = workspace.tabs[0]?.id ?? null;
        }
      });
      select(null);
    });
  const tools: LibraryTool[] =
    asset.kind === 'text'
      ? ['text', 'speech', 'voices', 'results']
      : asset.kind === 'subtitle'
        ? ['text', 'translate', 'speech', 'voices', 'results']
        : ['source', 'results'];
  return (
    <aside
      aria-label="Выбранный материал"
      className="h-full overflow-auto border-l border-border bg-surface p-4"
    >
      <p className="mb-3 text-xs uppercase text-muted">Выбранный материал</p>
      <h2 className="break-words text-lg font-semibold">{asset.name}</h2>
      <p className="mb-4 text-xs text-muted">{kindNames[asset.kind]}</p>
      <div className="space-y-2">
        {tools.map((tool) => (
          <button
            key={tool}
            className={`${button} w-full text-left`}
            onClick={() =>
              void action(() =>
                openAsset(asset, tool, browser.scope === 'project' ? browser.projectId : null),
              )
            }
            disabled={busy}
          >
            {toolNames[tool]}
          </button>
        ))}
      </div>
      <div className="mt-5 space-y-3 border-t border-border pt-4">
        <label className="block text-xs">
          Название
          <input
            className={field}
            value={name}
            onChange={(e) => setName(e.target.value)}
            maxLength={200}
          />
        </label>
        <label className="block text-xs">
          Теги через запятую
          <input
            className={field}
            value={tags}
            onChange={(e) => setTags(e.target.value)}
            maxLength={2000}
          />
        </label>
        <button className={button} onClick={() => void save()} disabled={busy || !name.trim()}>
          Сохранить свойства
        </button>
      </div>
      <AssetVersions asset={asset} />
      <p className="mt-5 text-xs text-muted">
        Используется:{' '}
        {links.length
          ? links
              .map((l) => projects.find((p) => p.id === l.projectId)?.title ?? l.projectId)
              .join(', ')
          : 'без проекта'}
      </p>
      <label className="mt-3 block text-xs">
        Проект
        <select
          aria-label="Проект"
          className={field}
          value={target}
          onChange={(e) => {
            setTarget(e.target.value);
          }}
        >
          <option value="">Выберите проект</option>
          {projects
            .filter(
              (p) =>
                !state.standaloneOwners.includes(p.id) && !state.removedProjects.includes(p.id),
            )
            .map((p) => (
              <option key={p.id} value={p.id}>
                {p.title}
              </option>
            ))}
        </select>
      </label>
      <button
        className={`${button} mt-2 w-full`}
        onClick={() => void attach()}
        disabled={busy || !target}
      >
        Добавить копию в проект
      </button>
      {browser.projectId && (
        <>
          <button
            className={`${button} mt-2 w-full`}
            disabled={busy}
            onClick={() =>
              void action(() =>
                mutate((draft) => {
                  draft.links = draft.links.filter(
                    (l) => l.assetId !== asset.id || l.projectId !== browser.projectId,
                  );
                }),
              )
            }
          >
            Убрать из проекта
          </button>
          <button
            className={`${button} mt-2 w-full`}
            disabled={busy}
            onClick={() =>
              void action(async () => {
                await copyAsset(
                  asset,
                  { projectId: browser.projectId, folderId: null },
                  mutate,
                  refresh,
                );
              })
            }
          >
            Независимая копия
          </button>
        </>
      )}
      <div className="mt-5 border-t border-border pt-3">
        {confirmDelete ? (
          <>
            <p className="text-xs">
              Убрать материал из библиотеки и всех проектов? Сохранённые файлы останутся для
              восстановления.
            </p>
            <button className={button} onClick={() => void remove()} disabled={busy}>
              Подтвердить удаление
            </button>
            <button className={button} onClick={() => setConfirmDelete(false)}>
              Отмена
            </button>
          </>
        ) : (
          <button className={`${button} w-full`} onClick={() => setConfirmDelete(true)}>
            Удалить из библиотеки…
          </button>
        )}
      </div>
      {error && (
        <p role="alert" className="mt-3 text-sm">
          {error}
        </p>
      )}
    </aside>
  );
}
