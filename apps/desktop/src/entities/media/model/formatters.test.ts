import { describe, expect, it } from 'vitest';
import { formatProjectTitle, formatSourceLabel } from './formatters';

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
});
