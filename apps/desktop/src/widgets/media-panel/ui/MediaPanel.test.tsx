// @vitest-environment jsdom
import { afterEach, describe, expect, it } from 'vitest';
import { cleanup, render, screen } from '@testing-library/react';
import { ProjectContext, type Project } from '@/entities/project';
import { MediaPanel } from './MediaPanel';

const project: Project = {
  id: 'p-1',
  title: 'Long media project',
  status: 'ready_for_processing',
  source: { kind: 'managedLocalFile', artifactId: 'a-1', originalFilename: 'local-video.mp4' },
  metadata: {
    durationMs: 123000,
    width: 3840,
    height: 2160,
    fps: 59.94,
    videoCodec: 'h264',
    container: 'mp4',
    hasVideo: true,
    hasAudio: true,
    audioTracks: [],
    streams: [],
  },
  createdAt: '2026-01-01T00:00:00Z',
  updatedAt: '2026-01-01T00:00:00Z',
};

describe('MediaPanel', () => {
  afterEach(() => cleanup());

  it('does not force a fixed desktop width on its root panel', () => {
    render(
      <ProjectContext.Provider value={createProjectContext(project)}>
        <MediaPanel />
      </ProjectContext.Provider>,
    );

    const panel = screen.getByLabelText('Media details');

    expect(panel.className).toContain('min-w-0');
    expect(panel.className).not.toContain('w-80');
    expect(panel.className).not.toContain('shrink-0');
  });

  it('shows a safe local source filename without exposing the full path', () => {
    render(
      <ProjectContext.Provider
        value={createProjectContext({
          ...project,
          source: {
            kind: 'externalLocalFile',
            path: 'C:\\Users\\person\\Videos\\private-folder\\clip.mp4',
          },
        })}
      >
        <MediaPanel />
      </ProjectContext.Provider>,
    );

    expect(screen.getByLabelText('Source: clip.mp4')).not.toBeNull();
    expect(document.body.innerHTML).not.toContain('Users\\person');
  });

  it('keeps a safe source label visible when metadata is not available yet', () => {
    render(
      <ProjectContext.Provider
        value={createProjectContext({
          ...project,
          metadata: null,
          source: {
            kind: 'externalLocalFile',
            path: 'C:\\Users\\person\\Videos\\private-folder\\clip.mp4',
          },
        })}
      >
        <MediaPanel />
      </ProjectContext.Provider>,
    );

    expect(screen.getByText('No metadata available')).not.toBeNull();
    expect(screen.getByLabelText('Source: clip.mp4')).not.toBeNull();
    expect(document.body.innerHTML).not.toContain('Users\\person');
  });
});

function createProjectContext(projectValue: Project) {
  return {
    projectId: projectValue.id,
    project: projectValue,
    setProjectId: () => {},
    setProject: () => {},
    deletingProjectId: null,
    beginProjectDeletion: () => false,
    finishProjectDeletion: () => {},
    operationGeneration: 0,
    captureToken: () => ({ generation: 0, projectId: projectValue.id }),
    validateToken: () => true,
  };
}
