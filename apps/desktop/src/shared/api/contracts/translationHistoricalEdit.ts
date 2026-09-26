import type { TranslationLink, TranslationPublication } from './translation';

export interface HistoricalTranslationEditContext {
  projectId: string;
  translationId: string;
  baseResultId: string;
  runId: string;
  baseRevision: number;
  expectedHeadResultId: string;
  headRevision: number;
  expectedLinkRevision: number;
}

export interface EditHistoricalTranslationResult {
  projectId: string;
  translationId: string;
  baseResultId: string;
  resultId: string;
  expectedHeadResultId: string;
  expectedLinkRevision: number;
  segmentId: number;
  lines: string[];
}

export interface TranslationPublicationView {
  link: TranslationLink;
  publication: TranslationPublication;
}
