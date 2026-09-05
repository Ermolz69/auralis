import { beforeEach, describe, expect, it, vi } from 'vitest';
import { invoke } from '@/shared/api/tauri/invoke';
import type { MediaMetadata } from '../model/types';
import { importLocalMedia, probeLocalMedia } from './mediaApi';

vi.mock('@/shared/api/tauri/invoke', () => ({
  invoke: vi.fn(),
}));

const metadata: MediaMetadata = {
  durationMs: 42_000,
  hasVideo: true,
  hasAudio: false,
  audioTracks: [],
  streams: [],
};

describe('mediaApi', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('probes the selected local path', async () => {
    vi.mocked(invoke).mockResolvedValueOnce(metadata as never);

    await expect(probeLocalMedia('C:\\media\\clip.mp4')).resolves.toBe(metadata);
    expect(invoke).toHaveBeenCalledExactlyOnceWith('probe_local_media_cmd', {
      path: 'C:\\media\\clip.mp4',
    });
  });

  it('imports the selected path into the requested project', async () => {
    const importedProject = { id: 'project-1' };
    vi.mocked(invoke).mockResolvedValueOnce(importedProject as never);

    await expect(importLocalMedia('project-1', 'C:\\media\\clip.mp4')).resolves.toBe(
      importedProject,
    );
    expect(invoke).toHaveBeenCalledExactlyOnceWith('import_local_media_cmd', {
      projectId: 'project-1',
      path: 'C:\\media\\clip.mp4',
    });
  });
});
