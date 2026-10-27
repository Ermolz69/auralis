import { useEffect, useState } from 'react';
import type { LibraryAsset } from '@/shared/api/contracts/library';
import type { Project } from '@/shared/api/contracts/project';
import { invoke } from '@/shared/api/tauri';

export function useSourceOwnership(project: Project | null, asset?: LibraryAsset) {
  const [downloadedId, setDownloadedId] = useState<string | null>(null);
  useEffect(() => {
    let active = true;
    setDownloadedId(null);
    if (project?.source?.kind === 'youtubeUrl' && asset?.ownerProjectId === project.id)
      void invoke('list_project_artifacts_cmd', { projectId: project.id, kind: 'downloadedVideo' })
        .then((artifacts) => {
          if (
            active &&
            artifacts.some((item) => item.id === asset.artifactId && item.state === 'ready')
          )
            setDownloadedId(asset.artifactId);
        })
        .catch(() => {});
    return () => {
      active = false;
    };
  }, [project?.id, project?.source?.kind, asset?.ownerProjectId, asset?.artifactId]);
  if (!asset) return true;
  if (!project || asset.ownerProjectId !== project.id) return false;
  if (!asset.artifactId) return asset.id === project.id;
  return project.source?.kind === 'managedLocalFile'
    ? project.source.artifactId === asset.artifactId
    : downloadedId === asset.artifactId;
}
