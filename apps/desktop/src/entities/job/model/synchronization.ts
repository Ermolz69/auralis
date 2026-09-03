import { listJobs, subscribeJobEvents, subscribeJobsInvalidated } from '../api/jobApi';
import type { UnlistenFn } from '@tauri-apps/api/event';
import type { Dispatch } from 'react';
import type { JobStoreAction } from './reducer';
import { DEFAULT_JOB_SYNCHRONIZATION_CONFIG } from './types';
import { validateJobEventDto, validateJobSnapshot } from './validation';
import { toCommandError } from '@/shared/api/contracts';

export class JobStoreSynchronizer {
  private unlistenJobs: UnlistenFn | null = null;
  private unlistenInvalidation: UnlistenFn | null = null;
  private activeGeneration: number = 0;

  // Single-flight and readiness
  private fetchInProgress = false;
  private activeFetchGeneration: number | null = null;
  private pendingFetch = false;
  private listenersReadyGeneration: number | null = null;

  // Backoff timers
  private listenerRetryTimer: ReturnType<typeof setTimeout> | null = null;
  private snapshotRetryTimer: ReturnType<typeof setTimeout> | null = null;
  private listenerRetryAttempt = 0;
  private snapshotRetryAttempt = 0;

  private dispatch: Dispatch<JobStoreAction>;
  constructor(dispatch: Dispatch<JobStoreAction>) {
    this.dispatch = dispatch;
  }

  public async startCycle() {
    const newGeneration = ++this.activeGeneration;
    const expectedGen = newGeneration;

    this.cleanupCycle();

    this.listenersReadyGeneration = null;
    this.activeFetchGeneration = null;
    this.fetchInProgress = false;
    this.pendingFetch = false;
    this.listenerRetryAttempt = 0;
    this.snapshotRetryAttempt = 0;

    this.dispatch({ type: 'INITIALIZATION_CYCLE', generation: expectedGen });

    await this.registerListeners(expectedGen);
  }

  public dispose() {
    this.activeGeneration++;
    this.cleanupCycle();
    this.listenersReadyGeneration = null;
    this.activeFetchGeneration = null;
    this.fetchInProgress = false;
    this.pendingFetch = false;
  }

  private cleanupCycle() {
    this.cleanupListeners();
    this.clearTimers();
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
    if (this.listenerRetryTimer !== null) {
      clearTimeout(this.listenerRetryTimer);
      this.listenerRetryTimer = null;
    }
    if (this.snapshotRetryTimer !== null) {
      clearTimeout(this.snapshotRetryTimer);
      this.snapshotRetryTimer = null;
    }
  }

  private async registerListeners(expectedGen: number) {
    if (this.activeGeneration !== expectedGen) return;

    this.cleanupListeners();
    let unlistenJ: UnlistenFn | null = null;
    let unlistenInv: UnlistenFn | null = null;

    try {
      unlistenJ = await subscribeJobEvents((event) => {
        this.handleEvent(event, expectedGen);
      });
      if (this.activeGeneration !== expectedGen) {
        if (unlistenJ) unlistenJ();
        return;
      }

      unlistenInv = await subscribeJobsInvalidated(() => {
        this.handleInvalidation(expectedGen);
      });
      if (this.activeGeneration !== expectedGen) {
        if (unlistenJ) unlistenJ();
        if (unlistenInv) unlistenInv();
        return;
      }

      this.unlistenJobs = unlistenJ;
      this.unlistenInvalidation = unlistenInv;
      this.listenersReadyGeneration = expectedGen;
      this.listenerRetryAttempt = 0;

      this.dispatch({ type: 'LISTENERS_REGISTERED', generation: expectedGen });

      this.performFetch(expectedGen);
    } catch (err) {
      console.warn('JobStore: Failed to register Tauri listeners', toCommandError(err));
      if (unlistenJ) unlistenJ();
      if (unlistenInv) unlistenInv();

      if (this.activeGeneration === expectedGen) {
        this.dispatch({ type: 'LISTENERS_FAILED', generation: expectedGen });
        this.scheduleListenerRetry(expectedGen);
      }
    }
  }

  private scheduleListenerRetry(expectedGen: number) {
    if (this.listenerRetryTimer !== null) {
      clearTimeout(this.listenerRetryTimer);
    }

    const exponent = Math.min(
      this.listenerRetryAttempt,
      DEFAULT_JOB_SYNCHRONIZATION_CONFIG.retryExponentLimit,
      30,
    );
    const delay = Math.min(
      DEFAULT_JOB_SYNCHRONIZATION_CONFIG.retryInitialMs * Math.pow(2, exponent),
      DEFAULT_JOB_SYNCHRONIZATION_CONFIG.retryMaxMs,
    );
    this.listenerRetryAttempt++;

    this.listenerRetryTimer = setTimeout(() => {
      this.listenerRetryTimer = null;
      if (this.activeGeneration !== expectedGen) return;
      void this.registerListeners(expectedGen);
    }, delay);
  }

  private handleEvent(payload: unknown, expectedGen: number) {
    if (this.activeGeneration !== expectedGen) return;

    if (!validateJobEventDto(payload)) {
      console.warn('JobStore: Received invalid job event');
      this.dispatch({ type: 'INVALIDATION_RECEIVED', generation: expectedGen });
      this.requestFetch(expectedGen);
      return;
    }
    this.dispatch({ type: 'EVENT_RECEIVED', event: payload, generation: expectedGen });
  }

  private handleInvalidation(expectedGen: number) {
    if (this.activeGeneration !== expectedGen) return;

    this.dispatch({ type: 'INVALIDATION_RECEIVED', generation: expectedGen });
    this.requestFetch(expectedGen);
  }

  public requestFetch(expectedGeneration: number) {
    if (expectedGeneration !== this.activeGeneration) {
      return;
    }
    this.performFetch(expectedGeneration);
  }

  private async performFetch(expectedGen: number) {
    if (this.activeGeneration !== expectedGen) return;

    if (this.listenersReadyGeneration !== expectedGen) {
      this.pendingFetch = true;
      return;
    }

    // Coalesce refresh requests during backoff instead of retrying a failing backend immediately.
    if (this.snapshotRetryTimer !== null) {
      this.pendingFetch = true;
      return;
    }

    if (this.fetchInProgress && this.activeFetchGeneration === expectedGen) {
      this.pendingFetch = true;
      return;
    }

    this.fetchInProgress = true;
    this.activeFetchGeneration = expectedGen;
    this.pendingFetch = false;

    try {
      this.dispatch({ type: 'CLEAR_PENDING_REFETCH', generation: expectedGen });
      this.dispatch({ type: 'FETCH_STARTED', generation: expectedGen });

      const snapshot = await listJobs();

      if (this.activeGeneration !== expectedGen) {
        return;
      }

      if (!validateJobSnapshot(snapshot)) {
        throw new Error('Invalid snapshot payload or duplicate jobs');
      }

      this.snapshotRetryAttempt = 0;

      this.dispatch({
        type: 'SNAPSHOT_RESOLVED',
        generation: expectedGen,
        jobs: snapshot,
      });
    } catch (err) {
      console.error('JobStore: Snapshot fetch failed', toCommandError(err));

      if (this.activeGeneration === expectedGen) {
        this.dispatch({ type: 'FETCH_FAILED', generation: expectedGen });
        this.scheduleSnapshotRetry(expectedGen);
      }
    } finally {
      if (this.activeGeneration === expectedGen) {
        this.fetchInProgress = false;
        this.activeFetchGeneration = null;

        if (this.pendingFetch && this.snapshotRetryTimer === null) {
          this.pendingFetch = false;
          this.performFetch(expectedGen);
        }
      }
    }
  }

  private scheduleSnapshotRetry(expectedGen: number) {
    if (this.snapshotRetryTimer !== null) {
      clearTimeout(this.snapshotRetryTimer);
    }

    const exponent = Math.min(
      this.snapshotRetryAttempt,
      DEFAULT_JOB_SYNCHRONIZATION_CONFIG.retryExponentLimit,
      30,
    );
    const delay = Math.min(
      DEFAULT_JOB_SYNCHRONIZATION_CONFIG.retryInitialMs * Math.pow(2, exponent),
      DEFAULT_JOB_SYNCHRONIZATION_CONFIG.retryMaxMs,
    );
    this.snapshotRetryAttempt++;

    this.snapshotRetryTimer = setTimeout(() => {
      this.snapshotRetryTimer = null;
      if (this.activeGeneration !== expectedGen) return;
      void this.performFetch(expectedGen);
    }, delay);
  }
}
