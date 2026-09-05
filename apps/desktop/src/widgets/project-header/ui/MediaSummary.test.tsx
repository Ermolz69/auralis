// @vitest-environment jsdom
import { cleanup, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import type { MediaMetadata } from '@/entities/media';
import { MediaSummary } from './MediaSummary';

afterEach(() => cleanup());

const baseMetadata: MediaMetadata = {
  durationMs: 125_000,
  hasVideo: true,
  hasAudio: true,
  audioTracks: [],
  streams: [],
};

describe('MediaSummary', () => {
  it('summarizes core media properties and prefers the default audio track', () => {
    render(
      <MediaSummary
        metadata={{
          ...baseMetadata,
          width: 1920,
          height: 1080,
          container: 'mp4',
          videoCodec: 'h264',
          audioTracks: [
            { streamIndex: 1, codec: 'opus', isDefault: false },
            {
              streamIndex: 2,
              codec: 'aac',
              channels: 2,
              sampleRate: 48_000,
              language: 'en',
              isDefault: true,
            },
          ],
        }}
      />,
    );

    expect(screen.getByText('2:05')).not.toBeNull();
    expect(screen.getByText('1920×1080')).not.toBeNull();
    expect(screen.getByText('MP4')).not.toBeNull();
    expect(screen.getByText('H264')).not.toBeNull();
    expect(screen.getByText('2 audio tracks')).not.toBeNull();
    expect(screen.getByText('AAC / 2ch / 48000Hz / en')).not.toBeNull();
  });

  it('uses the first audio track and safe fallbacks when details are missing', () => {
    render(
      <MediaSummary
        metadata={{
          ...baseMetadata,
          audioTracks: [{ streamIndex: 1, isDefault: false }],
        }}
      />,
    );

    expect(screen.getByText('UNKNOWN')).not.toBeNull();
    expect(screen.getByText('1 audio track')).not.toBeNull();
    expect(screen.getByText('UNKNOWN / ?ch / ?Hz / und')).not.toBeNull();
  });

  it('omits unavailable dimensions, codecs, and audio details', () => {
    const { container } = render(<MediaSummary metadata={{ ...baseMetadata, audioTracks: [] }} />);

    expect(screen.getByText('UNKNOWN')).not.toBeNull();
    expect(container.textContent).not.toContain('audio track');
    expect(container.textContent).not.toContain('×');
  });
});
