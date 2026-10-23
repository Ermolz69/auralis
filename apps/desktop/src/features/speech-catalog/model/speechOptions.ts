import type { SpeechVoiceOption } from '@/shared/api/contracts/speechCatalog';
import type { SpeechInput } from './speechDraft';
export type SpeechWorkflowOptions = SpeechInput & {
  projectId: string;
  profileId: string;
  voiceId: string;
  referenceId: string;
  voices: SpeechVoiceOption[];
  engineId: string;
};
