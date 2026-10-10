import type { CommandMap } from './commandMap';
import type { Validator } from './validationPrimitives';
import { isArrayOf, isNull } from './validationPrimitives';
import {
  validateTranslationComparisonPage,
  validateTranslationInspection,
  validateTranslationLink,
  validateTranslationPackageStatus,
  validateTranslationPublication,
  validateTranslationRunSnapshot,
  validateTranslationStart,
  isUuid,
} from './translationValidation';
import { validateTranslationHistoryPage } from './translationHistoryValidation';
import { validateTranslationReviewView } from './translationReviewValidation';
import {
  validateHistoricalTranslationEditContext,
  validateTranslationPublicationView,
} from './translationHistoricalValidation';

export const translationCommandResultValidators = {
  inspect_translation_source_cmd: validateTranslationInspection,
  begin_translation_run_cmd: validateTranslationLink,
  start_translation_run_cmd: validateTranslationStart,
  get_translation_package_status_cmd: validateTranslationPackageStatus,
  install_translation_package_cmd: validateTranslationPackageStatus,
  select_translation_package_cmd: validateTranslationPackageStatus,
  edit_translation_result_cmd: isUuid,
  get_historical_translation_edit_context_cmd: validateHistoricalTranslationEditContext,
  edit_historical_translation_result_cmd: isUuid,
  get_translation_publication_cmd: validateTranslationPublicationView,
  publish_translation_result_cmd: validateTranslationPublication,
  list_project_translations_cmd: (value) => isArrayOf(value, validateTranslationLink),
  get_translation_comparison_cmd: validateTranslationComparisonPage,
  get_translation_run_status_cmd: validateTranslationRunSnapshot,
  list_translation_history_cmd: validateTranslationHistoryPage,
  select_translation_result_cmd: validateTranslationLink,
  get_translation_review_cmd: validateTranslationReviewView,
  record_translation_review_cmd: validateTranslationReviewView,
  pause_translation_run_cmd: isNull,
} satisfies Partial<Record<keyof CommandMap, Validator>>;
