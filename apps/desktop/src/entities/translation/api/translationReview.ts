import { invoke } from '@/shared/api/tauri';
import type {
  TranslationReviewDecision,
  TranslationReviewView,
} from '@/shared/api/contracts/translation';

export function getTranslationReview(
  projectId: string,
  translationId: string,
  resultId: string,
): Promise<TranslationReviewView> {
  return invoke('get_translation_review_cmd', { projectId, translationId, resultId });
}

export function recordTranslationReview(
  projectId: string,
  translationId: string,
  resultId: string,
  view: TranslationReviewView,
  decision: TranslationReviewDecision,
  reviewerLabel: string,
  note: string,
): Promise<TranslationReviewView> {
  return invoke('record_translation_review_cmd', {
    projectId,
    translationId,
    resultId,
    expectedResultSha256: view.resultSha256,
    expectedLinkRevision: view.linkRevision,
    decision,
    reviewerLabel,
    note,
  });
}
