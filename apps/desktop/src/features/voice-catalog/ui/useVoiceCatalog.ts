import { convertFileSrc } from '@tauri-apps/api/core';
import { useEffect, useRef, useState } from 'react';
import type {
  CastRole,
  CastResult,
  ManualSelection,
  VoiceCatalogEntry,
  VoiceDetails,
  VoiceReferenceMetadata,
} from '@/shared/api/contracts/caster';
import { invoke } from '@/shared/api/tauri';
import { describeCommandError } from '@/shared/api/contracts/error';

export function useVoiceCatalog(projectId: string) {
  const [voices, setVoices] = useState<VoiceCatalogEntry[]>([]);
  const [nextVoice, setNextVoice] = useState<string | null>(null);
  const [voice, setVoice] = useState<VoiceDetails | null>(null);
  const [playback, setPlayback] = useState<string | null>(null);
  const [cast, setCast] = useState<CastResult | null>(null);
  const [roles, setRoles] = useState<Record<string, CastRole>>({});
  const [selections, setSelections] = useState<Record<string, ManualSelection>>({});
  const [roleId, setRoleId] = useState('narrator');
  const [roleName, setRoleName] = useState('Рассказчик');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const requestId = useRef<string | null>(null);

  useEffect(() => {
    let active = true;
    setError(null);
    void Promise.all([
      invoke('list_voice_catalog_cmd', { projectId, after: null, limit: 20 }),
      invoke('get_manual_cast_cmd', { projectId }),
    ])
      .then(([page, current]) => {
        if (!active) return;
        setVoices(page.items);
        setNextVoice(page.next_after);
        setCast(current);
        setRoles(current?.revision.cast.roles ?? {});
        setSelections(
          Object.fromEntries(
            Object.entries(current?.revision.selections ?? {}).map(([id, item]) => [
              id,
              {
                voice_id: item.voice_id,
                reference_id: item.reference_id,
                engine_id: item.engine_id,
                emotion_id: item.emotion_id,
              },
            ]),
          ),
        );
      })
      .catch((reason: unknown) => {
        if (active) setError(message(reason));
      });
    return () => {
      active = false;
    };
  }, [projectId]);

  const loadMoreVoices = async () => {
    if (!nextVoice || busy) return;
    setBusy(true);
    try {
      const page = await invoke('list_voice_catalog_cmd', {
        projectId,
        after: nextVoice,
        limit: 20,
      });
      setVoices((existing) => [...existing, ...page.items]);
      setNextVoice(page.next_after);
    } catch (reason) {
      setError(message(reason));
    } finally {
      setBusy(false);
    }
  };

  const inspectVoice = async (id: string, afterReference: string | null = null) => {
    setBusy(true);
    setError(null);
    try {
      const page = await invoke('inspect_voice_cmd', {
        projectId,
        voiceId: id,
        afterReference,
        limit: 20,
      });
      setVoice((previous) =>
        afterReference && previous?.voice_id === id
          ? {
              ...page,
              references: [...previous.references, ...page.references],
            }
          : page,
      );
      if (!afterReference) setPlayback(null);
    } catch (reason) {
      setError(message(reason));
    } finally {
      setBusy(false);
    }
  };

  const inspectAudio = async (reference: VoiceReferenceMetadata) => {
    if (!voice) return;
    setBusy(true);
    setError(null);
    try {
      const inspected = await invoke('inspect_voice_reference_cmd', {
        projectId,
        voiceId: voice.voice_id,
        referenceId: reference.reference_id,
      });
      setPlayback(convertFileSrc(inspected.playback_path));
    } catch (reason) {
      setError(message(reason));
    } finally {
      setBusy(false);
    }
  };

  const chooseReference = async (reference: VoiceReferenceMetadata) => {
    if (!voice || reference.qualification_status !== 'qualified') return;
    if (!/^[A-Za-z0-9_-]{1,64}$/.test(roleId) || !roleName.trim()) {
      setError('Укажите ID роли латиницей и её название.');
      return;
    }
    setBusy(true);
    setError(null);
    try {
      const compatible = await invoke('check_voice_compatibility_cmd', {
        projectId,
        voiceId: voice.voice_id,
        engineId: reference.engine_id,
        emotionId: reference.emotion_id,
      });
      if (!compatible.compatible || !compatible.reference_ids.includes(reference.reference_id)) {
        throw new Error('Голос не подтверждён для выбранного движка и эмоции.');
      }
      setRoles((existing) => ({
        ...existing,
        [roleId]: { display_name: roleName.trim(), voice_id: voice.voice_id },
      }));
      setSelections((existing) => ({
        ...existing,
        [roleId]: {
          voice_id: voice.voice_id,
          reference_id: reference.reference_id,
          engine_id: reference.engine_id,
          emotion_id: reference.emotion_id,
        },
      }));
      requestId.current = null;
      setNotice(`Для роли ${roleId} выбран ${voice.display_name}. Сохраните ревизию.`);
    } catch (reason) {
      setError(message(reason));
    } finally {
      setBusy(false);
    }
  };

  const saveCast = async () => {
    if (busy || Object.keys(selections).length === 0) return;
    setBusy(true);
    setError(null);
    setNotice(null);
    requestId.current ??= crypto.randomUUID();
    try {
      const saved = await invoke('assign_manual_cast_cmd', {
        projectId,
        cast: { version: 2, roles },
        selections,
        expectedRevision: cast?.revision_sha256 ?? null,
        requestId: requestId.current,
      });
      setCast(saved);
      requestId.current = null;
      setNotice(`Ревизия ${saved.revision_sha256.slice(0, 12)} сохранена.`);
    } catch (reason) {
      setError(message(reason));
    } finally {
      setBusy(false);
    }
  };

  const recover = async () => {
    setBusy(true);
    setError(null);
    try {
      await invoke('recover_manual_cast_cmd', { projectId });
      const current = await invoke('get_manual_cast_cmd', { projectId });
      setCast(current);
      setNotice('Ревизия восстановлена.');
    } catch (reason) {
      setError(message(reason));
    } finally {
      setBusy(false);
    }
  };

  return {
    voices,
    nextVoice,
    voice,
    playback,
    cast,
    roles,
    selections,
    roleId,
    roleName,
    busy,
    error,
    notice,
    setRoleId,
    setRoleName,
    loadMoreVoices,
    inspectVoice,
    inspectAudio,
    chooseReference,
    saveCast,
    recover,
  };
}

function message(reason: unknown): string {
  return describeCommandError(reason, 'Операция с голосами не выполнена');
}
