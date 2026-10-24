import { useCallback, useEffect, useRef, useState, type ReactNode } from 'react';
import { invoke, listen } from '@/shared/api/tauri';
import { describeCommandError, toCommandError } from '@/shared/api/contracts/error';
import {
  emptyLibrary,
  type LibraryAsset,
  type LibraryState,
  type LibraryTool,
} from '@/shared/api/contracts/library';
import type { Project } from '@/shared/api/contracts/project';
import { defaultTool, newWorkspace, openTab, workspaceKey } from './catalog';
import { LibraryContext } from './context';

export function LibraryProvider({ children }: { children: ReactNode }) {
  const [state, setState] = useState(emptyLibrary);
  const current = useRef(state);
  const queue = useRef(Promise.resolve());
  const [projects, setProjects] = useState<Project[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [validationWarning, setValidationWarning] = useState<string | null>(null);
  const retryProject = useRef<string | null>(null);
  const [view, setView] = useState<'library' | 'workspace' | 'settings'>('library');
  const [workspaceId, setWorkspaceId] = useState<string | null>(null);
  const publish = (next: LibraryState) => {
    current.current = next;
    setState(next);
  };
  const enqueue = useCallback((work: () => Promise<void>) => {
    const job = queue.current.then(work);
    queue.current = job.catch(() => {});
    return job.catch((reason) => {
      setError(describeCommandError(reason));
      throw reason;
    });
  }, []);
  const refresh = useCallback(
    () =>
      enqueue(async () => {
        try {
          const [catalog, list] = await Promise.all([
            invoke('get_library_cmd'),
            invoke('list_projects_cmd'),
          ]);
          publish(catalog);
          setProjects(list);
          setError(null);
          if (retryProject.current) {
            try {
              await invoke('validate_media_project_cmd', { projectId: retryProject.current });
              retryProject.current = null;
              setValidationWarning(null);
            } catch (reason) {
              setValidationWarning(toCommandError(reason).message);
            }
          }
        } finally {
          setLoading(false);
        }
      }),
    [enqueue],
  );
  useEffect(() => {
    void refresh().catch(() => {});
  }, [refresh]);
  useEffect(() => {
    let timer: ReturnType<typeof setTimeout>;
    let active = true;
    const schedule = () => {
      clearTimeout(timer);
      timer = setTimeout(() => void refresh().catch(() => {}), 250);
    };
    const subscription = Promise.all([
      listen('project-updated', schedule),
      listen('job-event', ({ payload }) => {
        if (['completed', 'failed', 'cancelled'].includes(payload.kind)) schedule();
      }),
    ]).then((stops) => () => stops.forEach((stop) => stop()));
    void subscription
      .then((stop) => {
        if (!active) stop();
      })
      .catch(() => {});
    return () => {
      active = false;
      clearTimeout(timer);
      void subscription.then((stop) => stop()).catch(() => {});
    };
  }, [refresh]);
  useEffect(() => {
    if (workspaceId && !state.workspaces.some((w) => w.id === workspaceId)) {
      setWorkspaceId(null);
      setView('library');
    }
  }, [state.workspaces, workspaceId]);
  const mutate = useCallback(
    (change: (draft: LibraryState) => void) =>
      enqueue(async () => {
        for (let attempt = 0; attempt < 2; attempt++) {
          const draft = structuredClone(current.current);
          change(draft);
          try {
            publish(await invoke('save_library_cmd', { state: draft }));
            setError(null);
            return;
          } catch (reason) {
            if (attempt || toCommandError(reason).code !== 'CONFLICT') throw reason;
            publish(await invoke('get_library_cmd'));
          }
        }
      }),
    [enqueue],
  );
  const openProject = async (id: string) => {
    try {
      await invoke('validate_media_project_cmd', { projectId: id });
      setValidationWarning(null);
      retryProject.current = null;
    } catch (reason) {
      retryProject.current = id;
      setValidationWarning(toCommandError(reason).message);
    }
    const key = workspaceKey(id, null);
    await mutate((draft) => {
      if (!draft.workspaces.some((w) => w.id === key))
        draft.workspaces.push(newWorkspace(id, null));
    });
    setWorkspaceId(key);
    setView('workspace');
  };
  const openAsset = async (
    asset: LibraryAsset,
    tool?: LibraryTool,
    contextProjectId?: string | null,
  ) => {
    const projectId =
      contextProjectId === undefined
        ? current.current.browser.scope === 'project'
          ? current.current.browser.projectId
          : null
        : contextProjectId;
    const key = workspaceKey(projectId, projectId ? null : asset.id);
    try {
      await invoke('validate_media_project_cmd', { projectId: projectId ?? asset.ownerProjectId });
      setValidationWarning(null);
      retryProject.current = null;
    } catch (reason) {
      retryProject.current = projectId ?? asset.ownerProjectId;
      setValidationWarning(toCommandError(reason).message);
    }
    await mutate((draft) => {
      let workspace = draft.workspaces.find((w) => w.id === key);
      if (!workspace) {
        workspace = newWorkspace(projectId, projectId ? null : asset.id);
        draft.workspaces.push(workspace);
      }
      const stored = draft.assets.find((a) => a.id === asset.id);
      if (stored) stored.openedAt = new Date().toISOString();
      openTab(workspace, tool ?? defaultTool(asset), asset.id);
    });
    setWorkspaceId(key);
    setView('workspace');
  };
  const openTool = async (tool: LibraryTool, assetId?: string | null) => {
    await mutate((draft) => {
      const workspace = draft.workspaces.find((w) => w.id === workspaceId);
      if (workspace) openTab(workspace, tool, assetId === undefined ? workspace.assetId : assetId);
    });
  };
  const activateWorkspace = (id: string) => {
    if (current.current.workspaces.some((w) => w.id === id)) {
      setWorkspaceId(id);
      setView('workspace');
    }
  };
  return (
    <LibraryContext.Provider
      value={{
        state,
        projects,
        loading,
        error: error ?? validationWarning,
        view,
        workspaceId,
        setView,
        activateWorkspace,
        mutate,
        refresh,
        openProject,
        openAsset,
        openTool,
      }}
    >
      {children}
    </LibraryContext.Provider>
  );
}
