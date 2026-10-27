import { useCallback, useState } from 'react';
import { TranslationReview } from '@/features/translation-review';
import { TranslationModelSetup } from '@/features/translation-model-setup';
import { TranslationRunControl } from '@/features/translation-run-control';
import { TranslationSourceImport } from '@/features/translation-source-import';
import { useProjectContext } from '@/entities/project';
import type { LibraryAsset } from '@/shared/api/contracts/library';
export function TranslateTool({ asset }: { asset?: LibraryAsset }) {
  const { projectId } = useProjectContext();
  const [revision, setRevision] = useState(0);
  const [results, setResults] = useState(0);
  const refreshResults = useCallback(() => setResults((v) => v + 1), []);
  if (!projectId) return <p role="status">Загружаем контекст перевода…</p>;
  return (
    <div className="space-y-5">
      <h2 className="text-xl font-semibold">Перевод субтитров</h2>
      <TranslationModelSetup />
      <TranslationSourceImport
        sourceArtifactId={asset?.artifactId ?? undefined}
        projectId={projectId}
        onTranslationPrepared={() => setRevision((v) => v + 1)}
      />
      <TranslationRunControl
        sourceArtifactId={asset?.artifactId ?? undefined}
        key={`${projectId}:${revision}`}
        projectId={projectId}
        onReadyResultsChanged={refreshResults}
      />
      <TranslationReview
        sourceArtifactId={asset?.artifactId ?? undefined}
        refreshRevision={results}
      />
    </div>
  );
}
