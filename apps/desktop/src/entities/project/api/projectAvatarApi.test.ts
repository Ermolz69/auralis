import { beforeEach, describe, expect, it, vi } from 'vitest';
import { invoke } from '@/shared/api/tauri';
import { getProjectAvatar, setProjectAvatar } from './projectAvatarApi';

vi.mock('@/shared/api/tauri', () => ({
  invoke: vi.fn(),
}));

describe('projectAvatarApi', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('loads the avatar for the requested project', async () => {
    const avatar = { dataUrl: 'data:image/webp;base64,YXZhdGFy', initialized: true };
    vi.mocked(invoke).mockResolvedValueOnce(avatar as never);

    await expect(getProjectAvatar('project-1')).resolves.toBe(avatar);
    expect(invoke).toHaveBeenCalledExactlyOnceWith('get_project_avatar_cmd', {
      projectId: 'project-1',
    });
  });

  it('overwrites an avatar by default', async () => {
    await setProjectAvatar('project-1', 'data:image/webp;base64,bmV3');

    expect(invoke).toHaveBeenCalledExactlyOnceWith('set_project_avatar_cmd', {
      projectId: 'project-1',
      dataUrl: 'data:image/webp;base64,bmV3',
      onlyIfMissing: false,
    });
  });

  it('supports clearing and initialize-only updates', async () => {
    await setProjectAvatar('project-1', null, true);

    expect(invoke).toHaveBeenCalledExactlyOnceWith('set_project_avatar_cmd', {
      projectId: 'project-1',
      dataUrl: null,
      onlyIfMissing: true,
    });
  });
});
