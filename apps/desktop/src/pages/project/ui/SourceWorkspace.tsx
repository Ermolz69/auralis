import { useMemo } from 'react';
import { useJobContext } from '@/entities/job';
import { useProjectContext } from '@/entities/project';
import { SourceActivityLog } from './source-workspace/SourceActivityLog';
import { SourceMetadata } from './source-workspace/SourceMetadata';
import { SourceSelector } from './source-workspace/SourceSelector';
import { buildSourceActivity, getSourceValue } from './source-workspace/model';

export function SourceWorkspace() {
  const { project } = useProjectContext();
  const { jobs } = useJobContext();
  const source = project?.source ?? null;
  const metadata = project?.metadata ?? null;
  const sourceValue = getSourceValue(source);
  const activity = useMemo(
    () => buildSourceActivity(project, Object.values(jobs)),
    [jobs, project],
  );
  return (
    <section
      className="h-full min-h-0 overflow-y-auto px-4 py-5 sm:px-6"
      aria-label="Video source configuration"
      data-testid="source-workspace"
    >
      <div className="space-y-5">
        <SourceSelector source={source} sourceValue={sourceValue} />
        {metadata && <SourceMetadata metadata={metadata} sourceValue={sourceValue} />}
        <SourceActivityLog items={activity} />
      </div>
    </section>
  );
}
