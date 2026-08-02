// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from 'vitest';
import { cleanup, render, screen } from '@testing-library/react';
import type { Project } from '@/entities/project';
import { ProjectListRow } from './ProjectListRow';

const project: Project = {
  id: 'project-1',
  title: 'C:\\Users\\person\\Videos\\private-folder\\clip.mp4',
  status: 'failed',
  source: {
    kind: 'externalLocalFile',
    path: 'C:\\Users\\person\\Videos\\private-folder\\clip.mp4',
  },
  metadata: null,
  createdAt: '2026-08-02T00:00:00.000Z',
  updatedAt: '2026-08-02T00:00:01.000Z',
};

afterEach(() => cleanup());

describe('ProjectListRow', () => {
  it('uses safe title, status, and source labels without exposing full local paths', () => {
    render(
      <ProjectListRow
        project={project}
        isDeleting={false}
        isAnyDeleting={false}
        openButtonRef={vi.fn()}
        deleteButtonRef={vi.fn()}
        onOpen={vi.fn()}
        onDelete={vi.fn()}
      />,
    );

    expect(screen.getAllByText('clip.mp4')).toHaveLength(2);
    expect(screen.getByText('Needs attention')).not.toBeNull();
    expect(screen.queryByText('failed')).toBeNull();
    expect(document.body.innerHTML).not.toContain('Users\\person');
    expect(
      screen.getByRole('button', {
        name: 'Open clip.mp4. Status: Needs attention. Source: clip.mp4',
      }),
    ).not.toBeNull();
  });
});
