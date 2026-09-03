import { useEffect, useRef, useState } from 'react';
import {
  getLegacyProjectAvatar,
  getProjectAvatar,
  normalizeProjectAvatar,
  removeLegacyProjectAvatar,
  setProjectAvatar,
} from '@/entities/project';
import { toast } from '@/shared/ui/toast';

function clearLegacy(projectId: string) {
  if (getLegacyProjectAvatar(projectId) && !removeLegacyProjectAvatar(projectId).persisted) {
    toast.warning('Avatar saved, but its old local copy could not be removed.');
  }
}

export function useProjectAvatar(projectId: string, blocked: boolean) {
  const [avatar, setAvatar] = useState<string | null>(null);
  const [isSaving, setIsSaving] = useState(false);
  const generation = useRef(0);
  const saving = useRef(false);
  const blockedRef = useRef(blocked);
  blockedRef.current = blocked;

  useEffect(() => {
    const sequence = ++generation.current;
    setAvatar(null);
    setIsSaving(false);
    saving.current = false;
    const load = async () => {
      try {
        let record = await getProjectAvatar(projectId);
        if (sequence !== generation.current) return;
        const legacy = getLegacyProjectAvatar(projectId);
        if (!record.initialized && legacy) {
          record = await setProjectAvatar(projectId, legacy, true);
        }
        if (sequence !== generation.current) return;
        setAvatar(record.dataUrl);
        if (record.initialized) clearLegacy(projectId);
      } catch {
        if (sequence === generation.current)
          toast.warning('Could not load or migrate the project avatar. Project data is unchanged.');
      }
    };
    void load();
    return () => {
      generation.current += 1;
    };
  }, [projectId]);

  const updateAvatar = async (file: File | null) => {
    if (blockedRef.current || saving.current) return;
    saving.current = true;
    setIsSaving(true);
    const sequence = ++generation.current;
    try {
      const dataUrl = file ? await normalizeProjectAvatar(file) : null;
      if (sequence !== generation.current || blockedRef.current) return;
      const record = await setProjectAvatar(projectId, dataUrl);
      if (sequence !== generation.current) return;
      setAvatar(record.dataUrl);
      clearLegacy(projectId);
    } catch {
      if (sequence === generation.current)
        toast.warning(
          'Could not save the avatar. Choose a valid PNG, JPEG, WebP or GIF image up to 5 MiB and try again.',
        );
    } finally {
      if (sequence === generation.current) {
        saving.current = false;
        setIsSaving(false);
      }
    }
  };
  return { avatar, updateAvatar, isSaving };
}
