import { describe, expect, it } from 'vitest';
import { formatProjectTitle, formatSourceLabel, supportsSubtitleImport } from './formatters';

describe('media presentation formatters', () => {
  it('shows safe source labels for YouTube and remote sources', () => {
    expect(
      formatSourceLabel({ kind: 'youtubeUrl', url: 'https://www.youtube.com/watch?v=abc' }),
    ).toBe('YouTube source (youtube.com)');
    expect(formatSourceLabel({ kind: 'remoteUrl', url: 'https://videos.example.com/a/b' })).toBe(
      'Remote source (videos.example.com)',
    );
  });

  it('does not promote raw URL titles into project headings', () => {
    expect(
      formatProjectTitle('https://youtube.com/watch?v=abc', {
        kind: 'youtubeUrl',
        url: 'https://youtube.com/watch?v=abc',
      }),
    ).toBe('YouTube project');
  });

  it('limits subtitle import capability to URL-backed sources', () => {
    expect(supportsSubtitleImport({ kind: 'youtubeUrl', url: 'https://youtube.com/watch?v=abc' })).toBe(
      true,
    );
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
    expect(supportsSubtitleImport({ kind: 'externalLocalFile', path: 'C:\\media\\local.mp4' })).toBe(
      false,
    );
  });
});
