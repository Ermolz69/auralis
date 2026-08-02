import { useEffect, useRef } from 'react';
import { RunDubbing } from '../../../features/run-dubbing';
import {
  PageHeader,
  PageHeaderGroup,
  PageTitle,
  PageDescription,
  PageActions,
} from '../../../shared/ui/page-layout';
import { useProjectContext } from '@/entities/project';
import { useNavigation } from '@/shared/router';
import { Button } from '@/shared/ui/button';
import { Icon } from '@/shared/ui/icon';
import {
  formatProjectStatus,
  formatProjectTitle,
  formatSourceLabel,
  getProjectStatusTone,
} from '@/entities/media';
import { Badge } from '@/shared/ui/badge';
import { MediaSummary } from './MediaSummary';

export const ProjectHeader = () => {
  const { project } = useProjectContext();
  const { setCurrentView } = useNavigation();
  const titleRef = useRef<HTMLHeadingElement>(null);
  const displayTitle = project
    ? formatProjectTitle(project.title, project.source)
    : 'Loading Project...';
  const sourceLabel = formatSourceLabel(project?.source ?? null);

  useEffect(() => {
    if (project?.id) titleRef.current?.focus();
  }, [project?.id]);

  return (
    <PageHeader className="px-6 py-4 bg-surface border-b border-muted items-center min-w-0">
      <PageHeaderGroup className="min-w-0">
        <div className="flex min-w-0 items-center gap-3">
          <Button
            type="button"
            variant="ghost"
            size="sm"
            onClick={() => setCurrentView('home')}
            className="shrink-0"
            leftIcon={<Icon name="ArrowLeft" size="sm" />}
          >
            Projects
          </Button>
          <PageTitle
            ref={titleRef}
            tabIndex={-1}
            className="min-w-0 truncate !text-xl focus:outline-none focus-visible:ring-2 focus-visible:ring-focus"
            aria-label={displayTitle}
          >
            {displayTitle}
          </PageTitle>
          {project?.status && (
            <Badge variant={getProjectStatusTone(project.status)} size="sm" className="shrink-0">
              {formatProjectStatus(project.status)}
            </Badge>
          )}
        </div>
        {project?.metadata ? (
          <MediaSummary metadata={project.metadata} />
        ) : (
          <PageDescription className="!text-sm mt-1">{sourceLabel}</PageDescription>
        )}
      </PageHeaderGroup>
      <PageActions>
        <RunDubbing />
      </PageActions>
    </PageHeader>
  );
};
