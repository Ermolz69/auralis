// @vitest-environment jsdom
import { describe, expect, it } from 'vitest';
import { render, screen } from '@testing-library/react';
import { JobContext } from '@/entities/job';
import { JobQueuePanel } from './JobQueuePanel';

describe('JobQueuePanel', () => {
  it('does not force a fixed desktop width on its root panel', () => {
    render(
      <JobContext.Provider
        value={{
          phase: 'ready',
          scopeProjectId: 'p-1',
          jobs: {},
          buffer: [],
          pendingRefetch: false,
          generation: 0,
        }}
      >
        <JobQueuePanel />
      </JobContext.Provider>,
    );

    const panel = screen.getByLabelText('Job queue');

    expect(panel.className).toContain('min-w-0');
    expect(panel.className).not.toContain('w-96');
    expect(panel.className).not.toContain('shrink-0');
  });
});
