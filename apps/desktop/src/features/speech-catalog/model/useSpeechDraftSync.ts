import { useEffect } from 'react';
import type { SpeechDraft } from './speechDraft';
export function useSpeechDraftSync(value: SpeechDraft, change?: (draft: SpeechDraft) => void) {
  const serialized = JSON.stringify(value);
  useEffect(() => {
    change?.(JSON.parse(serialized) as SpeechDraft);
  }, [serialized, change]);
}
