import { useEffect, useState } from 'react';
import { invoke } from '@/shared/api/tauri';
import type { MarkerPreparation, SpeechSourceRequest } from '@/shared/api/contracts/speech';

export function useMarkerPreparation(projectId: string) {
  const [preparation, setPreparation] = useState<MarkerPreparation | null>(null);
  const [originalRequest, setOriginalRequest] = useState<SpeechSourceRequest | null>(null);
  const [applied, setApplied] = useState(false);
  useEffect(() => {
    setPreparation(null);
    setOriginalRequest(null);
    setApplied(false);
  }, [projectId]);
  const clear = () => {
    setPreparation(null);
    setOriginalRequest(null);
    setApplied(false);
  };
  const normalize = async (source: SpeechSourceRequest) => {
    const result = await invoke('prepare_speech_text_cmd', { projectId, source });
    setPreparation(result);
    setOriginalRequest(source);
    setApplied(false);
    return result;
  };
  const confirm = async () => {
    if (!applied || !preparation || !originalRequest)
      throw new Error('Сначала примените подготовленный текст');
    return invoke('review_prepared_speech_text_cmd', {
      projectId,
      source: originalRequest,
      preparation,
      confirmed: true,
    });
  };
  return { preparation, applied, normalize, confirm, clear, apply: () => setApplied(true) };
}
