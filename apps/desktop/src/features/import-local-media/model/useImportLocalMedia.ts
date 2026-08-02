import { useState, useRef, useLayoutEffect } from 'react';
import { open } from '@tauri-apps/plugin-dialog';
import { useProjectContext, createProject } from '@/entities/project';
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
    setProjectId,
    setProject,
    projectId,
    operationGeneration,
    captureToken,
    validateToken,
  } = useProjectContext();
  const { setCurrentView } = useNavigation();

  const latestAttemptRef = useRef(0);
  const activeAttemptRef = useRef<number | null>(null);

  useLayoutEffect(() => {
    setIsImporting(false);
    setStage('idle');
    activeAttemptRef.current = null;
    latestAttemptRef.current += 1;
  }, [operationGeneration, projectId]);

  const isBlockedByDeletion = deletingProjectId !== null;

  const handleImport = async () => {
    if (deletingProjectId !== null || isImporting) return;
    if (activeAttemptRef.current !== null) return;

    const token = captureToken();
    if (!validateToken(token)) return;

    const attemptId = ++latestAttemptRef.current;
    activeAttemptRef.current = attemptId;

    const ownsAttempt = () =>
      latestAttemptRef.current === attemptId && activeAttemptRef.current === attemptId;

    const isCurrentAttempt = () => ownsAttempt() && validateToken(token);

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

      if (!isCurrentAttempt()) return;

      if (!selected || typeof selected !== 'string') {
        setIsImporting(false);
        setStage('idle');
        activeAttemptRef.current = null;
        return;
      }

      const filename = selected.split(/[/\\]/).pop() || 'Local Video';
      setSourceLabel(filename);

      setStage('probing');
      const project = await createProject(filename);
      if (!isCurrentAttempt()) return;
      setDraftProject(project);

      setStage('importing');
      const updatedProject = await importLocalMedia(project.id, selected);
      if (!isCurrentAttempt()) return;

      setIsImporting(false);
      setStage('idle');
      activeAttemptRef.current = null;
      setDraftProject(null);

      setProjectId(updatedProject.id);
      setProject(updatedProject);
      setCurrentView('project');
    } catch (err: any) {
      if (!isCurrentAttempt()) return;
      const cmdErr = toCommandError(err);
      setError(cmdErr.message);
      console.error(cmdErr);
    } finally {
      if (ownsAttempt()) {
        activeAttemptRef.current = null;
        if (validateToken(token)) {
          setIsImporting(false);
          setStage('idle');
        }
      }
    }
  };

  const openDraftProject = () => {
    if (!draftProject) return;
    setProjectId(draftProject.id);
    setProject(draftProject);
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
