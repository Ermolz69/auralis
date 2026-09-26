import { useCallback, useState } from 'react';
import { TranslationRunControl } from '@/features/translation-run-control';
import { TranslationReviewPanel } from '@/features/translation-review';

export function TranslationHistoryWorkspace({ projectId }: { projectId: string }) {
  const [revision, setRevision] = useState(0);
  const refresh = useCallback(() => setRevision((value) => value + 1), []);
  return (
    <>
      <TranslationRunControl projectId={projectId} onReadyResultsChanged={refresh} />
      <TranslationReviewPanel projectId={projectId} refreshRevision={revision} />
    </>
  );
}
