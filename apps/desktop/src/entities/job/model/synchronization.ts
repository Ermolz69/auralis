import { listen } from '@tauri-apps/api/event';
import type { UnlistenFn } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import type { Dispatch } from 'react';
import type { JobStoreAction } from './reducer';
import { DEFAULT_JOB_SYNCHRONIZATION_CONFIG } from './types';
import type { JobStoreState } from './types';
import { validateJobEventDto, validateJobSnapshot } from './validation';

export class JobStoreSynchronizer {
  private unlistenJobs: UnlistenFn | null = null;
  private unlistenInvalidation: UnlistenFn | null = null;
  private activeGeneration: number = 0;
  private activeProjectId: string | null = null;

  // Debouncing & fetching
  private fetchInProgress = false;
  private chainFetchCount = 0;
  
  // Backoff timers
  private retryTimeoutId: ReturnType<typeof setTimeout> | null = null;
  private retryAttempt = 0;

  private dispatch: Dispatch<JobStoreAction>;
  private getState: () => JobStoreState;

  constructor(
    dispatch: Dispatch<JobStoreAction>,
    getState: () => JobStoreState
  ) {
    this.dispatch = dispatch;
    this.getState = getState;
  }

  public async startCycle(projectId: string | null) {
    this.cleanup(); 
    
    this.dispatch({ type: 'INITIALIZATION_CYCLE' });
    const state = this.getState();
    this.activeGeneration = state.generation;
    this.activeProjectId = projectId;
    this.chainFetchCount = 0;
    this.fetchInProgress = false;

    try {
      const [unlistenJ, unlistenInv] = await Promise.all([
        listen('job-lifecycle-event', (event) => {
          this.handleEvent(event.payload);
        }),
        listen('job-lifecycle-invalidated', () => {
          this.handleInvalidation();
        }),
      ]);

      if (this.activeGeneration !== state.generation || this.activeProjectId !== projectId) {
        unlistenJ();
        unlistenInv();
        return;
      }

      this.unlistenJobs = unlistenJ;
      this.unlistenInvalidation = unlistenInv;

      this.dispatch({ type: 'LISTENERS_REGISTERED' });
      this.triggerFetchLoop();
    } catch (err) {
      console.warn('JobStore: Failed to register Tauri listeners', err);
      this.cleanupListeners();
      this.dispatch({ type: 'LISTENERS_FAILED' });
      this.scheduleRetry();
    }
  }

  public dispose() {
    this.cleanup();
  }

  private cleanup() {
    this.cleanupListeners();
    this.clearTimers();
    this.activeGeneration = 0; 
    this.activeProjectId = null;
  }

  private cleanupListeners() {
    if (this.unlistenJobs) {
      this.unlistenJobs();
      this.unlistenJobs = null;
    }
    if (this.unlistenInvalidation) {
      this.unlistenInvalidation();
      this.unlistenInvalidation = null;
    }
  }

  private clearTimers() {
    if (this.retryTimeoutId !== null) {
      clearTimeout(this.retryTimeoutId);
      this.retryTimeoutId = null;
    }
  }

  private handleEvent(payload: unknown) {
    if (!validateJobEventDto(payload)) {
      console.warn('JobStore: Received invalid job event', payload);
      this.handleInvalidation();
      return;
    }
    this.dispatch({ type: 'EVENT_RECEIVED', event: payload });
  }

  private handleInvalidation() {
    this.dispatch({ type: 'INVALIDATION_RECEIVED' });
    this.triggerFetchLoop();
  }

  private async triggerFetchLoop() {
    if (this.fetchInProgress) return;
    
    const state = this.getState();
    if (state.phase === 'initializing' || state.phase === 'idle') {
      return; 
    }

    if (!state.pendingRefetch && state.phase !== 'synchronizing') {
      return; 
    }

    this.fetchInProgress = true;
    this.clearTimers();

    const expectedGen = this.activeGeneration;
    const expectedProj = this.activeProjectId;

    try {
      this.dispatch({ type: 'CLEAR_PENDING_REFETCH' });
      this.dispatch({ type: 'FETCH_STARTED' });

      if (!expectedProj) {
         throw new Error("Cannot fetch snapshot without a projectId");
      }

      const snapshot = await invoke('list_jobs_snapshot_cmd', { projectId: expectedProj });

      if (this.activeGeneration !== expectedGen) {
        this.fetchInProgress = false;
        return; 
      }

      if (!validateJobSnapshot(snapshot, expectedProj)) {
        throw new Error('Invalid snapshot payload or contained foreign/duplicate jobs');
      }

      this.retryAttempt = 0;
      this.chainFetchCount++;

      this.dispatch({
        type: 'SNAPSHOT_RESOLVED',
        generation: expectedGen,
        projectId: expectedProj,
        jobs: snapshot,
      });

      this.fetchInProgress = false;

      const postState = this.getState();
      if (postState.pendingRefetch && this.chainFetchCount < 2) {
        this.triggerFetchLoop();
      } else {
        this.chainFetchCount = 0; 
      }
    } catch (err) {
      console.error('JobStore: Snapshot fetch failed', err);
      
      if (this.activeGeneration === expectedGen) {
        this.dispatch({ type: 'FETCH_FAILED' });
        this.fetchInProgress = false;
        this.chainFetchCount = 0;
        this.scheduleRetry();
      }
    }
  }

  private scheduleRetry() {
    if (this.retryAttempt >= DEFAULT_JOB_SYNCHRONIZATION_CONFIG.retryMaxAttempts) {
      console.warn('JobStore: Max retry attempts reached. Waiting for explicit invalidation.');
      return;
    }

    const delay = Math.min(
      DEFAULT_JOB_SYNCHRONIZATION_CONFIG.retryInitialMs * Math.pow(2, this.retryAttempt),
      DEFAULT_JOB_SYNCHRONIZATION_CONFIG.retryMaxMs
    );

    this.retryAttempt++;
    this.retryTimeoutId = setTimeout(() => {
      this.retryTimeoutId = null;
      if (this.getState().phase === 'stale') {
         if (!this.unlistenJobs || !this.unlistenInvalidation) {
             this.startCycle(this.activeProjectId);
         } else {
             this.triggerFetchLoop();
         }
      }
    }, delay);
  }
}
