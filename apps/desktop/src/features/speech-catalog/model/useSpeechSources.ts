import { useEffect, useState } from 'react';
import { invoke } from '@/shared/api/tauri';
import type { TranslationLink } from '@/shared/api/contracts/translation';
import type { Artifact } from '@/shared/api/contracts/artifact';

export function useSpeechSources(projectId: string) {
  const [subtitleFiles, setSubtitleFiles] = useState<Artifact[]>([]);
  const [translations, setTranslations] = useState<TranslationLink[]>([]);
  const refreshSources = async () => {
    const files = await invoke('list_project_artifacts_cmd', {
      projectId,
      kind: 'originalSubtitle',
    });
    const links = await invoke('list_project_translations_cmd', { projectId });
    setSubtitleFiles(files);
    setTranslations(links);
  };
  useEffect(() => {
    let active = true;
    void invoke('list_project_translations_cmd', { projectId })
      .then((items) => {
        if (active) setTranslations(items);
      })
      .catch(() => {});
    void invoke('list_project_artifacts_cmd', { projectId, kind: 'originalSubtitle' })
      .then((items) => {
        if (active) setSubtitleFiles(items);
      })
      .catch(() => {});
    return () => {
      active = false;
    };
  }, [projectId]);
  return { subtitleFiles, translations, refreshSources };
}
