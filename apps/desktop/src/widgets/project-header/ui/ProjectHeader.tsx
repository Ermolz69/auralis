import { RunDubbing } from '../../../features/run-dubbing';
import {
  PageHeader,
  PageHeaderGroup,
  PageTitle,
  PageDescription,
  PageActions,
} from '../../../shared/ui/page-layout';

export const ProjectHeader = () => {
  return (
    <PageHeader className="px-6 py-4 bg-surface border-b border-muted items-center">
      <PageHeaderGroup>
        <PageTitle className="!text-xl">Project Title</PageTitle>
        <PageDescription className="!text-sm mt-1">Video ID: dQw4w9WgXcQ</PageDescription>
      </PageHeaderGroup>
      <PageActions>
        <RunDubbing />
      </PageActions>
    </PageHeader>
  );
};
