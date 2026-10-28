import { useLibrary, toolNames } from '@/entities/library';
import type { LibraryWorkspace } from '@/shared/api/contracts/library';
import type { ReactNode } from 'react';
import { panelKey } from '../model/panels';

export function WorkspaceTabs({
  workspace,
  close,
  add,
}: {
  workspace: LibraryWorkspace;
  close: (id: string) => void;
  add?: ReactNode;
}) {
  const { state, mutate } = useLibrary();
  const select = (id: string) =>
    void mutate((state) => {
      state.workspaces.find((item) => item.id === workspace.id)!.activeTab = id;
    }).catch(() => {});
  return (
    <div className="flex min-h-16 shrink-0 items-center gap-2 px-3">
      <div className="flex min-w-0 gap-2 overflow-x-auto py-3">
        <div
          role="tablist"
          className="contents"
          aria-label="Вкладки рабочего пространства"
          aria-owns={workspace.tabs.map((tab) => `tab-${tab.id}`).join(' ') || undefined}
        />
        {workspace.tabs.map((tab) => (
          <div
            key={tab.id}
            className={`flex shrink-0 items-center overflow-hidden rounded-[6px] border ${workspace.activeTab === tab.id ? 'border-border-strong bg-surface-active' : 'border-border bg-surface'}`}
          >
            <button
              role="tab"
              id={`tab-${tab.id}`}
              aria-controls={`panel-${workspace.id}-${panelKey(tab)}`}
              aria-selected={workspace.activeTab === tab.id}
              tabIndex={workspace.activeTab === tab.id ? 0 : -1}
              className={`rounded-l-[5px] px-3 py-2 text-sm hover:bg-surface-raised ${workspace.activeTab === tab.id ? 'font-semibold' : ''}`}
              onClick={() => select(tab.id)}
              onKeyDown={(event) => {
                if (!['ArrowLeft', 'ArrowRight', 'Home', 'End'].includes(event.key)) return;
                event.preventDefault();
                const index = workspace.tabs.findIndex((item) => item.id === tab.id);
                const next =
                  event.key === 'Home'
                    ? 0
                    : event.key === 'End'
                      ? workspace.tabs.length - 1
                      : (index + (event.key === 'ArrowLeft' ? -1 : 1) + workspace.tabs.length) %
                        workspace.tabs.length;
                const target = workspace.tabs[next].id;
                document.getElementById(`tab-${target}`)?.focus();
                select(target);
              }}
            >
              {toolNames[tab.tool]}
              {tab.assetId
                ? ` · ${state.assets.find((asset) => asset.id === tab.assetId)?.name ?? ''}`
                : ''}
            </button>
            <button
              className="self-stretch rounded-r-[5px] px-2 text-muted hover:bg-surface-raised hover:text-text"
              aria-label={`Закрыть ${toolNames[tab.tool]}`}
              onClick={() => close(tab.id)}
            >
              ×
            </button>
          </div>
        ))}
      </div>
      {add}
    </div>
  );
}
