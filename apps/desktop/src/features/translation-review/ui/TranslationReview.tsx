import { useProjectContext } from '@/entities/project';
import { TranslationReviewPanel } from './TranslationReviewPanel';

export function TranslationReview({ refreshRevision = 0 }: { refreshRevision?: number }) {
  const { project } = useProjectContext();
  return (
    <TranslationReviewPanel projectId={project?.id ?? null} refreshRevision={refreshRevision} />
  );
}
