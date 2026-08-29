// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, render, screen } from '@testing-library/react';
import type { Project } from '@/entities/project';
import { SourceWorkspace } from './SourceWorkspace';

const state = vi.hoisted(() => ({ project: null as Project | null, jobs: {} }));

vi.mock('@/entities/project', () => ({
  useProjectContext: () => ({ project: state.project }),
}));
vi.mock('@/entities/job', () => ({
  useJobContext: () => ({ jobs: state.jobs }),
}));
vi.mock('@/features/paste-youtube-link', () => ({
  PasteYoutubeLink: () => <button type="button">Подключить YouTube</button>,
}));
vi.mock('@/features/import-local-media', () => ({
  ImportLocalMediaButton: () => <button type="button">Выбрать локальный файл</button>,
}));

const baseProject: Project = {
  id: 'project-1',
  title: 'Demo',
  status: 'source_imported',
  source: null,
  metadata: null,
  createdAt: '2026-08-29T10:00:00.000Z',
  updatedAt: '2026-08-29T10:01:00.000Z',
};

beforeEach(() => {
  state.project = baseProject;
  state.jobs = {};
});
afterEach(() => cleanup());

describe('SourceWorkspace', () => {
  it('offers YouTube and local import when no source is connected', () => {
    render(<SourceWorkspace />);
    expect(screen.getByRole('button', { name: 'Подключить YouTube' })).not.toBeNull();
    expect(screen.getByRole('button', { name: 'Выбрать локальный файл' })).not.toBeNull();
  });

  it('shows a remote source as a labelled read-only URL without a misleading remove button', () => {
    state.project = {
      ...baseProject,
      source: { kind: 'remoteUrl', url: 'https://media.example/video.mp4' },
    };
    render(<SourceWorkspace />);

    const input = screen.getByRole('textbox', { name: 'Video URL' }) as HTMLInputElement;
    expect(input.readOnly).toBe(true);
    expect(input.value).toBe('https://media.example/video.mp4');
    expect(screen.queryByRole('button', { name: /remove|clear/i })).toBeNull();
  });

  it('shows a local source summary without an empty YouTube URL field', () => {
    state.project = {
      ...baseProject,
      source: { kind: 'managedLocalFile', artifactId: 'artifact-1', originalFilename: 'clip.mp4' },
    };
    render(<SourceWorkspace />);

    expect(screen.getByText('Локальный видеофайл')).not.toBeNull();
    expect(screen.getByText('clip.mp4')).not.toBeNull();
    expect(screen.queryByRole('textbox', { name: 'Video URL' })).toBeNull();
  });

  it('preserves metadata and exposes activity timestamps semantically', () => {
    state.project = {
      ...baseProject,
      source: { kind: 'youtubeUrl', url: 'https://youtube.com/watch?v=demo' },
      metadata: {
        durationMs: 62_000,
        width: 1920,
        height: 1080,
        fps: 25,
        videoCodec: 'h264',
        container: 'mp4',
        hasVideo: true,
        hasAudio: true,
        audioTracks: [
          { streamIndex: 1, codec: 'aac', channels: 2, language: 'ru', isDefault: true },
        ],
        streams: [{ index: 0, codecType: 'video', codecName: 'h264' }],
      },
    };
    const { container } = render(<SourceWorkspace />);

    expect(screen.getByText('1920×1080')).not.toBeNull();
    expect(screen.getByText(/AAC · Track #1 · 2 ch · RU/)).not.toBeNull();
    const times = [...container.querySelectorAll('time')];
    expect(times.length).toBeGreaterThan(0);
    expect(times.every((time) => Boolean(time.dateTime))).toBe(true);
  });
});
