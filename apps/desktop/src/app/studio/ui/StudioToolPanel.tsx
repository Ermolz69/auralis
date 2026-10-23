import { describeCommandError } from '@/shared/api/contracts/error';
import { lazy, Suspense, useEffect, useRef, useState } from 'react';
import { ProjectProvider, useProjectContext } from '@/entities/project';
import { useLibrary } from '@/entities/library';
import { invoke } from '@/shared/api/tauri';
import type { LibraryTab, LibraryWorkspace } from '@/shared/api/contracts/library';
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
    asset?.ownerProjectId ??
    workspace.projectId ??
    state.assets.find((a) => a.id === workspace.assetId)?.ownerProjectId;
  const [project, setProject] = useState<Project | null>(null);
  const [error, setError] = useState('');
  useEffect(() => {
    let active = true;
    if (ownerId)
      void invoke('get_project_cmd', { projectId: ownerId })
        .then((value) => {
          if (active) setProject(value);
        })
        .catch((e) => {
          if (active) setError(describeCommandError(e));
        });
    return () => {
      active = false;
    };
  }, [ownerId]);
  if (error)
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
  return (
    <ProjectProvider>
      <SelectProject project={project} />
      <div className="mx-auto w-full max-w-7xl p-4 sm:p-6">
        <Suspense fallback={<p role="status">Загружаем инструмент…</p>}>
          {tab.tool === 'source' && <SourceTool />}
          {tab.tool === 'translate' && <TranslateTool />}
          {(tab.tool === 'text' || tab.tool === 'speech' || tab.tool === 'voices') && (
            <SpeechTool tab={tab} workspace={workspace} asset={asset} projectId={project.id} />
          )}
          {tab.tool === 'results' && <ResultsTool asset={asset} projectId={project.id} />}
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
