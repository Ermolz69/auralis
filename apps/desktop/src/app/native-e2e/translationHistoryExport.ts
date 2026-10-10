import { exportTranslationResult, getTranslationReview } from '@/entities/translation';

type ExportPrefix = 'native-export' | 'native-reexport';

export async function exportHistoryResult(
  projectId: string,
  translationId: string,
  resultId: string,
  prefix: ExportPrefix,
): Promise<string> {
  const source = __NATIVE_E2E_SUBTITLE_PATH__;
  const format = /[\\/]source\.(srt|vtt)$/i.exec(source)?.[1]?.toLowerCase();
  if (!format) throw new Error('Native history export requires an owned subtitle fixture');
  const destination = source.replace(/source\.(srt|vtt)$/i, `${prefix}-${resultId}.${format}`);
  const review = await getTranslationReview(projectId, translationId, resultId);
  if (!review.needsReview || review.latest)
    throw new Error('Native history export must retain an unapproved draft');
  const receipt = await exportTranslationResult(projectId, translationId, review, destination);
  if (receipt.sizeBytes === 0) throw new Error('Native history export is empty');
  return receipt.destination;
}
