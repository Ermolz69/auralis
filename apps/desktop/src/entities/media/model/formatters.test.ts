import { describe, expect, it } from 'vitest';
import {
  formatProjectStatus,
  formatProjectTitle,
  formatSourceLabel,
  getProjectStatusTone,
  supportsSubtitleImport,
} from './formatters';

describe('media presentation formatters', () => {
  it('shows safe source labels for YouTube and remote sources', () => {
    expect(
      formatSourceLabel({ kind: 'youtubeUrl', url: 'https://www.youtube.com/watch?v=abc' }),
    ).toBe('YouTube source (youtube.com)');
    expect(formatSourceLabel({ kind: 'remoteUrl', url: 'https://videos.example.com/a/b' })).toBe(
      'Remote source (videos.example.com)',
    );
  });

  it('shows only safe filenames for local sources', () => {
    expect(
      formatSourceLabel({
        kind: 'managedLocalFile',
        artifactId: 'artifact-1',
        originalFilename: 'C:\\Users\\person\\Videos\\clip.mp4',
      }),
    ).toBe('clip.mp4');
    expect(
      formatSourceLabel({ kind: 'externalLocalFile', path: '/Users/person/Videos/clip.mov' }),
    ).toBe('clip.mov');
  });

  it('does not promote raw URL titles into project headings', () => {
    expect(
      formatProjectTitle('https://youtube.com/watch?v=abc', {
        kind: 'youtubeUrl',
        url: 'https://youtube.com/watch?v=abc',
      }),
    ).toBe('YouTube project');
    expect(formatProjectTitle('C:\\Users\\person\\Videos\\clip.mp4', null)).toBe(
      'Untitled Project',
    );
    expect(
      formatProjectTitle('', {
        kind: 'externalLocalFile',
        path: '/Users/person/Videos/clip.mov',
      }),
    ).toBe('Untitled Project');
  });

  it('maps project statuses to user-facing labels and semantic tones', () => {
    expect(formatProjectStatus('draft')).toBe('Draft');
    expect(formatProjectStatus('source_imported')).toBe('Source imported');
    expect(formatProjectStatus('ready_for_processing')).toBe('Ready for processing');
    expect(formatProjectStatus('processing')).toBe('Processing');
    expect(formatProjectStatus('completed')).toBe('Completed');
    expect(formatProjectStatus('failed')).toBe('Needs attention');
    expect(formatProjectStatus('cancelled')).toBe('Cancelled');

    expect(getProjectStatusTone('completed')).toBe('success');
    expect(getProjectStatusTone('failed')).toBe('danger');
    expect(getProjectStatusTone('processing')).toBe('primary');
    expect(getProjectStatusTone('cancelled')).toBe('warning');
  });

  it('limits subtitle import capability to URL-backed sources', () => {
    expect(
      supportsSubtitleImport({ kind: 'youtubeUrl', url: 'https://youtube.com/watch?v=abc' }),
    ).toBe(true);
    expect(supportsSubtitleImport({ kind: 'remoteUrl', url: 'https://videos.example.com/a' })).toBe(
      true,
    );
    expect(
      supportsSubtitleImport({
        kind: 'managedLocalFile',
        artifactId: 'artifact-1',
        originalFilename: 'local.mp4',
      }),
    ).toBe(false);
    expect(
      supportsSubtitleImport({ kind: 'externalLocalFile', path: 'C:\\media\\local.mp4' }),
    ).toBe(false);
  });
});
