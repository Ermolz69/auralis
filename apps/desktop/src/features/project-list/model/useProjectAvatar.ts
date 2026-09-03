import { useEffect, useRef, useState } from 'react';
import {
  getLegacyProjectAvatar,
  getProjectAvatar,
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
      const dataUrl = file ? await readAvatar(file) : null;
      if (sequence !== generation.current || blockedRef.current) return;
      const record = await setProjectAvatar(projectId, dataUrl);
      if (sequence !== generation.current) return;
      setAvatar(record.dataUrl);
      clearLegacy(projectId);
    } catch {
      if (sequence === generation.current)
        toast.warning(
          'Could not save the avatar. Choose a PNG, JPEG, WebP or GIF image up to 1 MiB and try again.',
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

function readAvatar(file: File): Promise<string> {
  if (
    file.size > 1024 * 1024 ||
    !['image/png', 'image/jpeg', 'image/webp', 'image/gif'].includes(file.type)
  ) {
    return Promise.reject(new Error('Unsupported avatar'));
  }
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onerror = () => reject(new Error('Could not read avatar'));
    reader.onabort = () => reject(new Error('Avatar reading was cancelled'));
    reader.onload = () =>
      typeof reader.result === 'string'
        ? resolve(reader.result)
        : reject(new Error('Invalid avatar data'));
    reader.readAsDataURL(file);
  });
}
