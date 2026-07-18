import type { JobStoreState, JobDto, JobEventDto } from './types';
import { DEFAULT_JOB_SYNCHRONIZATION_CONFIG } from './types';

export function initializeStore(projectId: string | null): JobStoreState {
  return {
    phase: 'idle',
    scopeProjectId: projectId,
    jobs: {},
    buffer: [],
    pendingRefetch: false,
    generation: 0,
  };
}

export type JobStoreAction =
  | { type: 'SWITCH_PROJECT'; projectId: string | null; generation: number }
  | { type: 'INITIALIZATION_CYCLE'; generation: number }
  | { type: 'LISTENERS_FAILED'; generation: number }
  | { type: 'LISTENERS_REGISTERED'; generation: number }
  | { type: 'FETCH_STARTED'; generation: number }
  | { type: 'SNAPSHOT_RESOLVED'; generation: number; projectId: string | null; jobs: JobDto[] }
  | { type: 'FETCH_FAILED'; generation: number }
  | { type: 'EVENT_RECEIVED'; event: JobEventDto; generation: number }
  | { type: 'INVALIDATION_RECEIVED'; generation: number }
  | { type: 'CLEAR_PENDING_REFETCH'; generation: number };

export function jobStoreReducer(state: JobStoreState, action: JobStoreAction): JobStoreState {
  const isStartingAction = action.type === 'SWITCH_PROJECT' || action.type === 'INITIALIZATION_CYCLE';
  
  if (isStartingAction) {
    if (action.generation <= state.generation) {
      return state;
    }
  } else {
    if (action.generation !== state.generation) {
      return state;
    }
  }

  switch (action.type) {
    case 'SWITCH_PROJECT':
      return {
        phase: 'idle',
        scopeProjectId: action.projectId,
        jobs: {},
        buffer: [],
        pendingRefetch: false,
        generation: action.generation,
      };

    case 'INITIALIZATION_CYCLE':
      return {
        ...state,
        phase: 'initializing',
        generation: action.generation,
        buffer: [],
        pendingRefetch: false,
      };

    case 'LISTENERS_FAILED':
      return {
        ...state,
        phase: 'stale',
      };

    case 'LISTENERS_REGISTERED':
      return {
        ...state,
        phase: 'synchronizing',
      };

    case 'FETCH_STARTED':
      return state;

    case 'SNAPSHOT_RESOLVED': {
      if (action.projectId !== state.scopeProjectId) {
        return state;
      }

      const newJobs: Record<string, JobDto> = {};
      for (const job of action.jobs) {
        newJobs[job.id] = job;
      }

      let hasGap = false;
      const replayedJobs = { ...newJobs };

      for (const event of state.buffer) {
        const jobId = event.job.id;
        const currentJob = replayedJobs[jobId];
        const currentRevision = currentJob ? currentJob.revision : 0;

        if (event.job.revision <= currentRevision) {
          continue;
        } else if (event.job.revision === currentRevision + 1 || currentRevision === 0) {
          replayedJobs[jobId] = event.job;
        } else {
          hasGap = true;
          break;
        }
      }

      if (hasGap) {
        return {
          ...state,
          phase: 'stale',
          jobs: replayedJobs,
          buffer: [],
          pendingRefetch: true,
        };
      }

      return {
        ...state,
        phase: 'ready',
        jobs: replayedJobs,
        buffer: [],
      };
    }

    case 'FETCH_FAILED':
      return {
        ...state,
        phase: 'stale',
      };

    case 'EVENT_RECEIVED': {
      if (action.event.job.projectId !== state.scopeProjectId) {
        return state;
      }

      if (state.phase === 'ready') {
        const jobId = action.event.job.id;
        const currentJob = state.jobs[jobId];
        const currentRevision = currentJob ? currentJob.revision : 0;

        if (action.event.job.revision <= currentRevision) {
          return state;
        } else if (action.event.job.revision === currentRevision + 1 || currentRevision === 0) {
          return {
            ...state,
            jobs: {
              ...state.jobs,
              [jobId]: action.event.job,
            },
          };
        } else {
          return {
            ...state,
            phase: 'stale',
            buffer: [],
            pendingRefetch: true,
          };
        }
      } else {
        const newBuffer = [...state.buffer, action.event];
        if (newBuffer.length > DEFAULT_JOB_SYNCHRONIZATION_CONFIG.maxBufferedEvents) {
          return {
            ...state,
            phase: 'stale',
            buffer: [],
            pendingRefetch: true,
          };
        }
        return {
          ...state,
          buffer: newBuffer,
        };
      }
    }

    case 'INVALIDATION_RECEIVED':
      return {
        ...state,
        pendingRefetch: true,
        phase: state.phase === 'ready' ? 'stale' : state.phase,
      };

    case 'CLEAR_PENDING_REFETCH':
      return {
        ...state,
        pendingRefetch: false,
      };

    default:
      return state;
  }
}
