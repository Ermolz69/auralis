import type { MarkerPreparation, SpeechSourceRequest } from '@/shared/api/contracts/speech';
export type SpeechDraft = {
  mode?: 'text' | 'subtitles';
  text?: string;
  subtitleId?: string;
  translationId?: string;
  selected?: string;
  preparation?: MarkerPreparation | null;
  applied?: boolean;
  originalRequest?: SpeechSourceRequest | null;
};
export type SpeechInput = {
  assetId?: string;
  initialSource?: SpeechSourceRequest;
  draft?: SpeechDraft;
  onDraftChange?: (draft: SpeechDraft) => void;
  stage?: 'text' | 'speech' | 'voices';
};
