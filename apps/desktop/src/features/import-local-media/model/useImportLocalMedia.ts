import { useState, useLayoutEffect } from 'react';
import { open } from '@tauri-apps/plugin-dialog';
import { useProjectContext, useProjectOperation, createProject } from '@/entities/project';
import { importLocalMedia } from '@/entities/media';
import { useNavigation } from '@/shared/router';
import { toCommandError } from '@/shared/api/contracts';
import type { Project } from '@/entities/project';

export type LocalImportStage = 'idle' | 'selecting' | 'probing' | 'importing';

export function useImportLocalMedia() {
  const [isImporting, setIsImporting] = useState(false);
  const [stage, setStage] = useState<LocalImportStage>('idle');
  const [error, setError] = useState<string | null>(null);
  const [draftProject, setDraftProject] = useState<Project | null>(null);
  const [sourceLabel, setSourceLabel] = useState<string | null>(null);
  const {
    deletingProjectId,
    setProject,
    projectId,
    project: currentProject,
    operationGeneration,
  } = useProjectContext();
  const { setCurrentView, setPipelineStep = () => undefined } = useNavigation();

  const operation = useProjectOperation();

  useLayoutEffect(() => {
    setIsImporting(false);
    setStage('idle');
  }, [operationGeneration, projectId]);

  const isBlockedByDeletion = deletingProjectId !== null;

  const handleImport = async () => {
    if (deletingProjectId !== null || isImporting) return;
    const attempt = operation.begin();
    if (!attempt) return;

    setIsImporting(true);
    setStage('selecting');
    setError(null);
    setDraftProject(null);
    setSourceLabel(null);

    try {
      const selected = await open({
        multiple: false,
        filters: [
          {
            name: 'Video',
            extensions: ['mp4', 'mkv', 'avi', 'mov', 'webm'],
          },
        ],
      });

      if (!operation.isCurrent(attempt)) return;

      if (!selected || typeof selected !== 'string') {
        setIsImporting(false);
        setStage('idle');
        operation.finish(attempt);
        return;
      }

      const filename = selected.split(/[/\\]/).pop() || 'Local Video';
      setSourceLabel(filename);

      setStage('probing');
      const project = currentProject ?? (await createProject(filename));
      if (!operation.isCurrent(attempt)) return;
      setDraftProject(project);

      setStage('importing');
      const updatedProject = await importLocalMedia(project.id, selected);
      if (!operation.isCurrent(attempt)) return;

      setIsImporting(false);
      setStage('idle');
      operation.finish(attempt);
      setDraftProject(null);
      setProject(updatedProject);
      setPipelineStep('source');
      setCurrentView('project');
    } catch (err: unknown) {
      if (!operation.isCurrent(attempt)) return;
      const cmdErr = toCommandError(err);
      setError(cmdErr.message);
      console.error(cmdErr);
    } finally {
      if (operation.finish(attempt)) {
        setIsImporting(false);
        setStage('idle');
      }
    }
  };

  const openDraftProject = () => {
    if (!draftProject) return;
    setProject(draftProject);
    setPipelineStep('source');
    setCurrentView('project');
  };

  return {
    handleImport,
    openDraftProject,
    isImporting,
    isBlockedByDeletion,
    stage,
    error,
    draftProject,
    sourceLabel,
  };
}
