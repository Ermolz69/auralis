export type TranslationFormat = 'srt' | 'vtt';

export interface TranslationPackageStatus {
  schemaVersion: 1;
  releaseId: string;
  backend: string;
  supported: boolean;
  installed: boolean;
  selected: boolean;
}

export interface TranslationSegment {
  id: number;
  ordinal: number;
  cueLabel: string | null;
  startMs: number;
  endMs: number;
  sourceLines: string[];
  textRanges: [number, number][];
}

export interface ComparedTranslationSegment {
  id: number;
  ordinal: number;
  cueLabel: string | null;
  startMs: number;
  endMs: number;
  sourceLines: string[];
  translatedLines: string[];
}

export interface TranslationComparisonPage {
  resultId: string;
  runId: string;
  needsReview: boolean;
  totalSegments: number;
  offset: number;
  segments: ComparedTranslationSegment[];
}

export interface BeginTranslationRun {
  projectId: string;
  sourceArtifactId: string;
  format: TranslationFormat;
  sourceLanguage: 'zh';
  translationId: string;
  runId: string;
}

export interface EditTranslationResult {
  projectId: string;
  translationId: string;
  baseResultId: string;
  resultId: string;
  segmentId: number;
  lines: string[];
}

export interface TranslationInspection {
  format: TranslationFormat;
  sourceSha256: string;
  sourceSize: number;
  parserVersion: number;
  segments: TranslationSegment[];
}

export interface TranslationLink {
  translationId: string;
  projectId: string;
  sourceArtifactId: string;
  sourceSha256: string;
  sourceFormat: 'srt' | 'vtt';
  sourceLanguage: 'zh' | 'ja';
  targetLanguage: 'ru';
  activeRunId: string | null;
  selectedResultId: string | null;
  outputArtifactId: string | null;
  revision: number;
  createdAt: string;
  updatedAt: string;
}

export interface TranslationPublication {
  translationId: string;
  resultId: string;
  runId: string;
  artifactId: string;
  sourceSha256: string;
  needsReview: boolean;
  state: 'pending' | 'ready' | 'failed';
}

export interface TranslationStart {
  jobId: string;
  runId: string;
  state: 'accepted';
}

export type TranslationRunPhase = 'requested' | 'running' | 'paused' | 'failed' | 'validated';

export interface TranslationRunSnapshot {
  runId: string;
  phase: TranslationRunPhase;
  pauseRequested: boolean;
  savedBlocks: number;
  totalBlocks: number;
  resultId: string | null;
  needsReview: boolean | null;
}
