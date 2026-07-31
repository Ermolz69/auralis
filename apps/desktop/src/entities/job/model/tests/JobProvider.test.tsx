// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, act } from '@testing-library/react';
import React, { useContext } from 'react';
import { JobProvider, JobContext } from '../JobProvider';

// Mock the synchronizer constructor and methods to spy on them
const mockConstructor = vi.fn();
const mockStartCycle = vi.fn();
const mockDispose = vi.fn();
const mockRequestFetch = vi.fn();

vi.mock('../synchronization', () => {
  return {
    JobStoreSynchronizer: class MockJobStoreSynchronizer {
      constructor(dispatch: any, getState: any) {
        mockConstructor(dispatch, getState);
      }
      startCycle(projectId: string | null) {
        mockStartCycle(projectId);
      }
      dispose() {
        mockDispose();
      }
      requestFetch(expectedGen: number) {
        mockRequestFetch(expectedGen);
      }
    },
  };
});

const TestConsumer = ({ onState }: { onState: (state: any) => void }) => {
  const state = useContext(JobContext);
  onState(state);
  return null;
};

describe('JobProvider React Integration', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('keeps the same synchronizer instance across state updates (stable instance)', async () => {
    let capturedState: any = null;

    const { rerender } = render(
      <JobProvider projectId="project-1">
        <TestConsumer
          onState={(s) => {
            capturedState = s;
          }}
        />
      </JobProvider>,
    );

    // Initial render
    expect(mockConstructor).toHaveBeenCalledTimes(1);
    expect(mockStartCycle).toHaveBeenCalledWith('project-1');

    // Get the dispatch passed to the synchronizer
    const dispatch = mockConstructor.mock.calls[0][0];

    // Trigger a state change by dispatching LISTENERS_REGISTERED (assuming it keeps generation)
    act(() => {
      dispatch({ type: 'LISTENERS_REGISTERED', generation: capturedState.generation });
    });

    // Verify state changed (phase should become synchronizing)
    expect(capturedState.phase).toBe('synchronizing');

    // Re-render the provider with the same project ID
    rerender(
      <JobProvider projectId="project-1">
        <TestConsumer
          onState={(s) => {
            capturedState = s;
          }}
        />
      </JobProvider>,
    );

    // The synchronizer MUST NOT have been recreated!
    expect(mockConstructor).toHaveBeenCalledTimes(1);
  });

  it('triggers startCycle and dispose on project switch and unmount', () => {
    const { rerender, unmount } = render(
      <JobProvider projectId="project-1">
        <div />
      </JobProvider>,
    );

    expect(mockStartCycle).toHaveBeenLastCalledWith('project-1');
    expect(mockDispose).toHaveBeenCalledTimes(0);

    // Switch project
    rerender(
      <JobProvider projectId="project-2">
        <div />
      </JobProvider>,
    );

    expect(mockDispose).toHaveBeenCalledTimes(1);
    expect(mockStartCycle).toHaveBeenLastCalledWith('project-2');

    // Unmount
    unmount();
    expect(mockDispose).toHaveBeenCalledTimes(2);
  });

  it('triggers requestFetch(generation) when pendingRefetch transitions to true', () => {
    let capturedState: any = null;

    render(
      <JobProvider projectId="project-1">
        <TestConsumer
          onState={(s) => {
            capturedState = s;
          }}
        />
      </JobProvider>,
    );

    const dispatch = mockConstructor.mock.calls[0][0];

    // Trigger state transition: set pendingRefetch to true
    act(() => {
      dispatch({ type: 'INVALIDATION_RECEIVED', generation: capturedState.generation });
    });

    expect(capturedState.pendingRefetch).toBe(true);
    expect(mockRequestFetch).toHaveBeenCalledWith(capturedState.generation);
  });

  it('supports React StrictMode effect setup -> cleanup -> setup replay', () => {
    // Simulate StrictMode mount
    const { unmount } = render(
      <React.StrictMode>
        <JobProvider projectId="project-1">
          <div />
        </JobProvider>
      </React.StrictMode>,
    );

    // StrictMode setup -> cleanup -> setup replay will result in:
    // 1. startCycle('project-1')
    // 2. dispose()
    // 3. startCycle('project-1')
    expect(mockStartCycle).toHaveBeenCalledTimes(2);
    expect(mockDispose).toHaveBeenCalledTimes(1);
    expect(mockStartCycle).toHaveBeenLastCalledWith('project-1');

    unmount();
    expect(mockDispose).toHaveBeenCalledTimes(2);
  });
});
