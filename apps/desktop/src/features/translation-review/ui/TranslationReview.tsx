import { useProjectContext } from '@/entities/project';
import { TranslationReviewPanel } from './TranslationReviewPanel';

export function TranslationReview({
  refreshRevision = 0,
  sourceArtifactId,
}: {
  refreshRevision?: number;
  sourceArtifactId?: string;
}) {
  const { project } = useProjectContext();
  return (
    <TranslationReviewPanel
      sourceArtifactId={sourceArtifactId}
      projectId={project?.id ?? null}
      refreshRevision={refreshRevision}
    />
  );
}
