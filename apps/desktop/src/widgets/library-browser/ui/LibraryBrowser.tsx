import { describeCommandError } from '@/shared/api/contracts/error';
import { useEffect, useRef, useState } from 'react';
import { useLibrary, visibleAssets, kindNames } from '@/entities/library';
import type { LibraryBrowser as Browser, LibraryKind } from '@/shared/api/contracts/library';
import { importFiles } from '@/entities/library';
import { LibraryNavigation } from './LibraryNavigation';
import { LibraryCreateDialog } from './LibraryCreateDialog';
import { AssetInspector } from './AssetInspector';
import { AssetCards } from './AssetCards';
import { LibraryProjectActions } from './LibraryProjectActions';
import { button, field } from './controls';
export function LibraryBrowser() {
  const { state, projects, mutate, refresh, openAsset, loading } = useLibrary();
  const [browser, setBrowser] = useState(state.browser);
  const hydrated = useRef(false);
  const [dialog, setDialog] = useState<'text' | 'project' | 'folder' | 'youtube' | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  const assets = visibleAssets(state, browser);
  const selected = assets.find((a) => a.id === browser.selectedId);
  useEffect(() => {
    if (
      !loading &&
      browser.projectId &&
      (!projects.some((project) => project.id === browser.projectId) ||
        state.removedProjects.includes(browser.projectId))
    )
      setBrowser((current) => ({
        ...current,
        scope: 'all',
        projectId: null,
        folderId: null,
        selectedId: null,
      }));
  }, [loading, browser.projectId, projects, state.removedProjects]);
  useEffect(() => {
    if (!loading && !hydrated.current) {
      hydrated.current = true;
      setBrowser(state.browser);
    }
  }, [loading, state.browser]);
  useEffect(() => {
    if (loading) return;
    const timer = setTimeout(
      () =>
        void mutate((draft) => {
          draft.browser = browser;
        }).catch(() => {}),
      350,
    );
    return () => clearTimeout(timer);
  }, [browser, loading, mutate]);
  const navigate = (patch: Partial<Browser>) => setBrowser((current) => ({ ...current, ...patch }));
  const destination = {
    projectId: browser.scope === 'project' ? browser.projectId : null,
    folderId: browser.folderId,
  };
  const importNow = async () => {
    setBusy(true);
    setError('');
    try {
      await importFiles(destination, mutate, refresh);
    } catch (e) {
      setError(describeCommandError(e));
    } finally {
      setBusy(false);
    }
  };
  const title =
    browser.scope === 'project'
      ? [
          projects.find((p) => p.id === browser.projectId)?.title,
          state.folders.find((f) => f.id === browser.folderId)?.name,
        ]
          .filter(Boolean)
          .join(' / ')
      : browser.scope === 'unassigned'
        ? 'Без проекта'
        : browser.scope === 'recent'
          ? 'Недавние'
          : 'Все материалы';
  return (
    <div
      className="grid h-full min-h-0 grid-cols-[170px_minmax(0,1fr)] xl:grid-cols-[210px_minmax(0,1fr)_280px]"
      data-testid="library-browser"
    >
      <LibraryNavigation browser={browser} navigate={navigate} create={setDialog} />
      <section className="min-w-0 overflow-auto p-4 sm:p-6" aria-label="Библиотека материалов">
        <div className="mb-5 flex flex-wrap items-center justify-between gap-3">
          <h1 className="text-2xl font-semibold">{title || 'Библиотека'}</h1>
          <div className="flex flex-wrap gap-2">
            <button className={button} disabled={busy} onClick={() => void importNow()}>
              {busy ? 'Импортируем…' : 'Импорт файлов'}
            </button>
            <button className={button} onClick={() => setDialog('youtube')}>
              YouTube
            </button>
            <button className={button} onClick={() => setDialog('text')}>
              Новый текст
            </button>
            <button className={button} onClick={() => void refresh().catch(() => {})}>
              Обновить
            </button>
          </div>
        </div>
        {browser.projectId && (
          <LibraryProjectActions
            key={`${browser.projectId}:${browser.folderId}`}
            browser={browser}
            navigate={navigate}
          />
        )}
        <div className="mb-5 flex flex-wrap gap-2">
          <input
            type="search"
            aria-label="Поиск материалов"
            placeholder="Название или теги…"
            className={`${field} min-w-40 flex-1`}
            value={browser.search}
            onChange={(e) => navigate({ search: e.target.value })}
          />
          <select
            aria-label="Тип материала"
            className={`${field} !w-auto`}
            value={browser.kind}
            onChange={(e) => navigate({ kind: e.target.value })}
          >
            <option value="">Все типы</option>
            {Object.entries(kindNames).map(([id, label]) => (
              <option key={id} value={id as LibraryKind}>
                {label}
              </option>
            ))}
          </select>
        </div>
        {error && (
          <p role="alert" className="mb-3">
            {error}
          </p>
        )}
        {loading ? (
          <p role="status">Загружаем библиотеку…</p>
        ) : assets.length ? (
          <AssetCards
            assets={assets}
            selectedId={browser.selectedId}
            select={(id) => navigate({ selectedId: id })}
            open={(asset) =>
              void openAsset(asset, undefined, destination.projectId).catch(() => {})
            }
          />
        ) : (
          <div className="border border-dashed border-border p-8 text-center text-muted">
            <p>Здесь пока нет материалов.</p>
            <p className="mt-2 text-sm">
              Импортируйте файл или создайте текст. Проект можно добавить позже.
            </p>
          </div>
        )}
        {selected && (
          <div className="mt-5 border border-border xl:hidden">
            <AssetInspector
              key={selected.id}
              asset={selected}
              browser={browser}
              select={(id) => navigate({ selectedId: id })}
            />
          </div>
        )}
        <p className="mt-5 text-xs text-muted">
          Материалов: {assets.length} · Двойной щелчок или Enter — открыть
        </p>
      </section>
      <div className="hidden min-h-0 xl:block">
        {selected ? (
          <AssetInspector
            key={selected.id}
            asset={selected}
            browser={browser}
            select={(id) => navigate({ selectedId: id })}
          />
        ) : (
          <aside className="h-full border-l border-border p-5 text-sm text-muted">
            Выберите материал для просмотра свойств и действий.
          </aside>
        )}
      </div>
      {dialog && (
        <LibraryCreateDialog
          kind={dialog}
          destination={destination}
          close={() => setDialog(null)}
        />
      )}
    </div>
  );
}
