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

  return (
    <PageHeader className="px-6 py-4 bg-surface border-b border-muted items-center">
      <PageHeaderGroup>
        <div className="flex items-center gap-3">
          <Button
            type="button"
            variant="ghost"
            size="sm"
            onClick={() => setCurrentView('home')}
            leftIcon={<Icon name="ArrowLeft" size="sm" />}
          >
            Projects
          </Button>
          <PageTitle className="!text-xl">
            {project ? formatProjectTitle(project.title, project.source) : 'Loading Project...'}
          </PageTitle>
          {project?.status && (
            <Badge variant={getProjectStatusTone(project.status)} size="sm">
              {formatProjectStatus(project.status)}
            </Badge>
          )}
        </div>
        {project?.metadata ? (
          <MediaSummary metadata={project.metadata} />
        ) : (
          <PageDescription className="!text-sm mt-1">
            {formatSourceLabel(project?.source ?? null)}
          </PageDescription>
        )}
      </PageHeaderGroup>
      <PageActions>
        <RunDubbing />
      </PageActions>
    </PageHeader>
  );
};
