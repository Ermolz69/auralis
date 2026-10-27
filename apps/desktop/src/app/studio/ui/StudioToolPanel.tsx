import { describeCommandError } from '@/shared/api/contracts/error';
import { lazy, Suspense, useEffect, useRef, useState } from 'react';
import { ProjectProvider, useProjectContext } from '@/entities/project';
import { useLibrary } from '@/entities/library';
import { invoke } from '@/shared/api/tauri';
import type {
  ToolMaterialContext,
  LibraryTab,
  LibraryWorkspace,
} from '@/shared/api/contracts/library';
import type { Project } from '@/shared/api/contracts/project';
const SourceTool = lazy(() =>
  import('./tools/SourceTool').then((m) => ({ default: m.SourceTool })),
);
const TranslateTool = lazy(() =>
  import('./tools/TranslateTool').then((m) => ({ default: m.TranslateTool })),
);
const SpeechTool = lazy(() =>
  import('./tools/SpeechTool').then((m) => ({ default: m.SpeechTool })),
);
const ResultsTool = lazy(() =>
  import('./tools/ResultsTool').then((m) => ({ default: m.ResultsTool })),
);
export function StudioToolPanel({
  tab,
  workspace,
}: {
  tab: LibraryTab;
  workspace: LibraryWorkspace;
}) {
  const { state } = useLibrary();
  const asset = state.assets.find((a) => a.id === tab.assetId);
  const ownerId =
    workspace.projectId ??
    asset?.ownerProjectId ??
    state.assets.find((a) => a.id === workspace.assetId)?.ownerProjectId;
  const [context, setContext] = useState<ToolMaterialContext | null>(null);
  const [project, setProject] = useState<Project | null>(null);
  const [error, setError] = useState('');
  useEffect(() => {
    let active = true;
    setError('');
    setProject(null);
    setContext(null);
    if (ownerId)
      void Promise.all([
        invoke('get_project_cmd', { projectId: ownerId }),
        asset
          ? invoke('get_tool_context_cmd', { projectId: ownerId, assetId: asset.id }).catch(
              (reason) => {
                if (active) setError(describeCommandError(reason));
                return null;
              },
            )
          : Promise.resolve(null),
      ])
        .then(([value, material]) => {
          if (active) {
            setProject(value);
            setContext(material);
          }
        })
        .catch((e) => {
          if (active) setError(describeCommandError(e));
        });
    return () => {
      active = false;
    };
  }, [ownerId, asset?.id, asset?.artifactId]);
  if (error && !project)
    return (
      <p role="alert" className="p-5">
        {error}
      </p>
    );
  if (!project)
    return (
      <p role="status" className="p-5">
        Загружаем контекст материала…
      </p>
    );
  const selected = asset && context ? { ...asset, artifactId: context.artifactId } : asset;
  return (
    <ProjectProvider>
      <SelectProject project={project} />
      <div className="mx-auto w-full max-w-7xl p-4 sm:p-6">
        {error && (
          <p role="alert" className="mb-4 text-danger">
            {error} Доступны сохранённые материалы и черновики; обработка требует проверки проекта.
          </p>
        )}
        <Suspense fallback={<p role="status">Загружаем инструмент…</p>}>
          {tab.tool === 'source' && <SourceTool asset={selected} />}
          {tab.tool === 'translate' && <TranslateTool asset={selected} />}
          {(tab.tool === 'text' || tab.tool === 'speech' || tab.tool === 'voices') && (
            <SpeechTool tab={tab} workspace={workspace} asset={selected} projectId={project.id} />
          )}
          {tab.tool === 'results' && <ResultsTool asset={selected} projectId={project.id} />}
        </Suspense>
      </div>
    </ProjectProvider>
  );
}
function SelectProject({ project }: { project: Project }) {
  const { setProject } = useProjectContext();
  const select = useRef(setProject);
  useEffect(() => {
    select.current(project);
  }, [project]);
  return null;
}
