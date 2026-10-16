import type {
  HistoricalTranslationEditContext,
  EditHistoricalTranslationResult,
  TranslationPublicationView,
} from './translationHistoricalEdit';
import type { Artifact } from './artifact';
import type {
  BeginTranslationRun,
  EditTranslationResult,
  TranslationComparisonPage,
  TranslationHistoryPage,
  TranslationFormat,
  TranslationInspection,
  TranslationLink,
  TranslationPackageStatus,
  TranslationPublication,
  TranslationRunSnapshot,
  ProjectTranslationSnapshot,
  TranslationReviewDecision,
  TranslationReviewView,
  TranslationExportReceipt,
  TranslationStart,
} from './translation';

export interface TranslationCommandMap {
  inspect_translation_source_cmd: {
    args: { projectId: string; sourceArtifactId: string; format: TranslationFormat };
    result: TranslationInspection;
  };
  import_translation_source_cmd: {
    args: { projectId: string; path: string; format: TranslationFormat };
    result: Artifact;
  };
  begin_translation_run_cmd: {
    args: { input: BeginTranslationRun };
    result: TranslationLink;
  };
  start_translation_run_cmd: {
    args: { projectId: string; translationId: string };
    result: TranslationStart;
  };
  get_translation_package_status_cmd: {
    args: undefined;
    result: TranslationPackageStatus;
  };
  install_translation_package_cmd: {
    args: undefined;
    result: TranslationPackageStatus;
  };
  select_translation_package_cmd: {
    args: { releaseId: string; backend: string };
    result: TranslationPackageStatus;
  };
  edit_translation_result_cmd: {
    args: { input: EditTranslationResult };
    result: string;
  };
  get_historical_translation_edit_context_cmd: {
    args: { projectId: string; translationId: string; baseResultId: string };
    result: HistoricalTranslationEditContext;
  };
  edit_historical_translation_result_cmd: {
    args: { input: EditHistoricalTranslationResult };
    result: string;
  };
  get_translation_publication_cmd: {
    args: { projectId: string; translationId: string; resultId: string };
    result: TranslationPublicationView;
  };
  publish_translation_result_cmd: {
    args: { projectId: string; translationId: string; resultId: string };
    result: TranslationPublication;
  };
  list_project_translations_cmd: {
    args: { projectId: string };
    result: TranslationLink[];
  };
  get_project_translation_snapshot_cmd: {
    args: { projectId: string };
    result: ProjectTranslationSnapshot;
  };
  get_translation_comparison_cmd: {
    args: {
      projectId: string;
      translationId: string;
      resultId: string;
      offset: number;
      limit: number;
    };
    result: TranslationComparisonPage;
  };
  get_translation_run_status_cmd: {
    args: { projectId: string; translationId: string };
    result: TranslationRunSnapshot;
  };
  list_translation_history_cmd: {
    args: {
      projectId: string;
      translationId: string;
      beforeResultId?: string | null;
      limit: number;
    };
    result: TranslationHistoryPage;
  };
  select_translation_result_cmd: {
    args: {
      projectId: string;
      translationId: string;
      resultId: string;
      expectedLinkRevision: number;
    };
    result: TranslationLink;
  };
  get_translation_review_cmd: {
    args: { projectId: string; translationId: string; resultId: string };
    result: TranslationReviewView;
  };
  record_translation_review_cmd: {
    args: {
      projectId: string;
      translationId: string;
      resultId: string;
      expectedResultSha256: string;
      expectedLinkRevision: number;
      decision: TranslationReviewDecision;
      reviewerLabel: string;
      note: string;
    };
    result: TranslationReviewView;
  };
  export_translation_result_cmd: {
    args: {
      projectId: string;
      translationId: string;
      resultId: string;
      expectedResultSha256: string;
      destination: string;
    };
    result: TranslationExportReceipt;
  };
  pause_translation_run_cmd: {
    args: { projectId: string; translationId: string };
    result: null;
  };
}
