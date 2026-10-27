import type { Artifact } from '@/shared/api/contracts/artifact';
import type { SpeechSourceRequest } from '@/shared/api/contracts/speech';

export function sourceRequest(
  mode: 'text' | 'subtitles',
  text: string,
  translationId: string,
  subtitleId: string,
  subtitle?: Artifact,
): SpeechSourceRequest {
  return {
    kind: mode === 'text' ? 'text' : translationId ? 'translation' : 'subtitles',
    text: mode === 'text' ? text : null,
    translation_id: translationId || null,
    artifact_id: mode === 'subtitles' && subtitleId ? subtitleId : null,
    format: subtitle
      ? subtitle.location.value.toLowerCase().endsWith('.vtt')
        ? 'vtt'
        : 'srt'
      : null,
  };
}
