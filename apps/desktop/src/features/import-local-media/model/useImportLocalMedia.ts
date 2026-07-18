import { useState, useRef, useLayoutEffect } from 'react';
import { open } from '@tauri-apps/plugin-dialog';
import { useProjectContext, createProject } from '@/entities/project';
import { importLocalMedia } from '@/entities/media';
import { useNavigation } from '@/shared/router';

export function useImportLocalMedia() {
  const [isImporting, setIsImporting] = useState(false);
  const [error, setError] = useState<string | null>(null);
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
      latestAttemptRef.current === attemptId &&
      activeAttemptRef.current === attemptId;

    const isCurrentAttempt = () => ownsAttempt() && validateToken(token);

    setIsImporting(true);
    setError(null);

    try {
      // 1. Open file dialog
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
        activeAttemptRef.current = null;
        return; // User cancelled
      }

      // 2. Extract filename for title
      const filename = selected.split(/[/\\]/).pop() || 'Local Video';

      // 3. Create a blank project
      const project = await createProject(filename);
      if (!isCurrentAttempt()) return;

      // 4. Import the media and probe
      const updatedProject = await importLocalMedia(project.id, selected);
      if (!isCurrentAttempt()) return;

      setIsImporting(false);
      activeAttemptRef.current = null;

      setProjectId(updatedProject.id);
      setProject(updatedProject);
      setCurrentView('project');
    } catch (err: any) {
      if (!isCurrentAttempt()) return;
      setError(err?.toString() || 'Failed to import local media');
      console.error(err);
    } finally {
      if (ownsAttempt()) {
        activeAttemptRef.current = null;
        if (validateToken(token)) {
          setIsImporting(false);
        }
      }
    }
  };

  return {
    handleImport,
    isImporting,
    isBlockedByDeletion,
    error,
  };
}

