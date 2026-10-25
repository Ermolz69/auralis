import type { SpeechReferenceOption } from '@/shared/api/contracts/speechCatalog';

export function referenceAllowed(reference: SpeechReferenceOption, technical = false) {
  return (
    reference.qualification_status === 'qualified' ||
    (technical &&
      reference.qualification_status === 'unverified' &&
      reference.emotion_id === 'neutral')
  );
}
