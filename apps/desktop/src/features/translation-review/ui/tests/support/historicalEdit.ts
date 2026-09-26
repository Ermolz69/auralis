import type {
  HistoricalTranslationEditContext,
  TranslationPublicationView,
} from '@/shared/api/contracts/translationHistoricalEdit';
import type {
  ComparedTranslationSegment,
  TranslationPublication,
} from '@/shared/api/contracts/translation';

export const id = (n: number) => `00000000-0000-4000-8000-${n.toString().padStart(12, '0')}`;
export const context: HistoricalTranslationEditContext = {
  projectId: id(1),
  translationId: id(2),
  baseResultId: id(3),
  runId: id(4),
  baseRevision: 1,
  expectedHeadResultId: id(5),
  headRevision: 2,
  expectedLinkRevision: 3,
};
export const segment: ComparedTranslationSegment = {
  id: 1,
  ordinal: 0,
  cueLabel: null,
  startMs: 1000,
  endMs: 2000,
  sourceLines: ['你好。'],
  translatedLines: ['Привет.'],
};
export const publication: TranslationPublication = {
  translationId: context.translationId,
  resultId: id(10),
  runId: context.runId,
  artifactId: id(6),
  sourceSha256: 'a'.repeat(64),
  needsReview: true,
  state: 'pending',
};
export const view: TranslationPublicationView = {
  publication,
  link: {
    projectId: context.projectId,
    translationId: context.translationId,
    sourceArtifactId: id(7),
    sourceSha256: publication.sourceSha256,
    sourceFormat: 'srt',
    sourceLanguage: 'zh',
    targetLanguage: 'ru',
    activeRunId: context.runId,
    selectedResultId: context.baseResultId,
    outputArtifactId: id(8),
    revision: 3,
    createdAt: '2026-09-26T00:00:00Z',
    updatedAt: '2026-09-26T00:00:00Z',
  },
};
export function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason: unknown) => void;
  const promise = new Promise<T>((accept, fail) => {
    resolve = accept;
    reject = fail;
  });
  return { promise, resolve, reject };
}
