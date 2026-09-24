export interface TranslationSegment {
  id: number;
  ordinal: number;
  cueLabel: string | null;
  startMs: number;
  endMs: number;
  sourceLines: string[];
  textRanges: [number, number][];
}

export interface TranslationInspection {
  format: 'srt' | 'vtt';
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
