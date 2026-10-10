import { exportTranslationResult, getTranslationReview } from '@/entities/translation';

type ExportPrefix = 'native-export' | 'native-reexport';

export async function exportHistoryResult(
  projectId: string,
  translationId: string,
  resultId: string,
  prefix: ExportPrefix,
): Promise<string> {
  const source = __NATIVE_E2E_SUBTITLE_PATH__;
  if (!/[\\/]source\.srt$/i.test(source))
    throw new Error('Native history export requires the owned SRT fixture');
  const destination = source.replace(/source\.srt$/i, `${prefix}-${resultId}.srt`);
  const review = await getTranslationReview(projectId, translationId, resultId);
  if (!review.needsReview || review.latest)
    throw new Error('Native history export must retain an unapproved draft');
  const receipt = await exportTranslationResult(projectId, translationId, review, destination);
  if (receipt.sizeBytes === 0) throw new Error('Native history export is empty');
  return receipt.destination;
}
