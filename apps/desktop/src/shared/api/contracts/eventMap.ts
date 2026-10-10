import type { JobEvent } from './job';

export interface EventMap {
  'job-event': JobEvent;
  'job-events-invalidated': null;
  'project-updated': { projectId: string };
  'transcript-ready': { projectId: string; jobId: string };
}
