import { useState } from 'react';
import { open } from '@tauri-apps/plugin-dialog';
import { useProjectContext, createProject } from '@/entities/project';
import { importLocalMedia } from '@/entities/media';

import { useNavigation } from '@/shared/router';

export function useImportLocalMedia() {
  const [isImporting, setIsImporting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const { setProjectId, setProject } = useProjectContext();
  const { setCurrentView } = useNavigation();

  const handleImport = async () => {
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

      if (!selected || typeof selected !== 'string') {
        return; // User cancelled
      }

      setIsImporting(true);

      // 2. Extract filename for title
      const filename = selected.split(/[/\\]/).pop() || 'Local Video';

      // 3. Create a blank project
      const project = await createProject(filename);

      // 4. Import the media and probe
      const updatedProject = await importLocalMedia(project.id, selected);

      // 5. Navigate to project
      setProjectId(updatedProject.id);
      setProject(updatedProject);
      setCurrentView('project');
    } catch (err: any) {
      setError(err?.toString() || 'Failed to import local media');
      console.error(err);
    } finally {
      setIsImporting(false);
    }
  };

  return {
    handleImport,
    isImporting,
    error,
  };
}
