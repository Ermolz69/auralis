import type { SpeechWorkflowOptions } from './speechOptions';
import { referenceAllowed } from './referenceAllowed';

export function assignmentReference(
  {
    voices,
    voiceId,
    referenceId,
    engineId,
    allowUnverifiedNeutral = false,
  }: Pick<
    SpeechWorkflowOptions,
    'voices' | 'voiceId' | 'referenceId' | 'engineId' | 'allowUnverifiedNeutral'
  >,
  assignedVoice: string,
  assignedReference?: string,
) {
  const preferredEmotion = voices
    .find((voice) => voice.voice_id === voiceId)
    ?.references.find((reference) => reference.reference_id === referenceId)?.emotion_id;
  const compatible = voices
    .find((voice) => voice.voice_id === assignedVoice)
    ?.references.filter(
      (reference) =>
        reference.engine_id === engineId && referenceAllowed(reference, allowUnverifiedNeutral),
    );
  return assignedReference
    ? compatible?.find((reference) => reference.reference_id === assignedReference)
    : (compatible?.find((reference) => reference.emotion_id === preferredEmotion) ??
        compatible?.find((reference) => reference.emotion_id === 'neutral') ??
        compatible?.[0]);
}
