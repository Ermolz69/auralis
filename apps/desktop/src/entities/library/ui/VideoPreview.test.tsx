import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import type { LibraryAsset } from '@/shared/api/contracts/library';
import { VideoPreview } from './VideoPreview';

const invoke = vi.hoisted(() => vi.fn());
vi.mock('@/shared/api/tauri', () => ({ invoke }));
vi.mock('@tauri-apps/api/core', () => ({ convertFileSrc: (path: string) => 'asset:' + path }));
const asset = (id: string, artifactId: string | null = id): LibraryAsset => ({
  id,
  ownerProjectId: 'owner-' + id,
  name: id,
  kind: 'video',
  artifactId,
  format: 'mp4',
  text: null,
  parentId: null,
  tags: [],
  openedAt: null,
});
let observers: Array<{
  callback: IntersectionObserverCallback;
  observe: ReturnType<typeof vi.fn>;
  disconnect: ReturnType<typeof vi.fn>;
}>;
beforeEach(() => {
  invoke.mockReset();
  observers = [];
  vi.stubGlobal(
    'IntersectionObserver',
    class {
      observe = vi.fn();
      disconnect = vi.fn();
      constructor(callback: IntersectionObserverCallback) {
        observers.push({ callback, observe: this.observe, disconnect: this.disconnect });
      }
    },
  );
});
afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});
function intersect(index = 0, visible = true) {
  act(() =>
    observers[index].callback(
      [{ isIntersecting: visible } as IntersectionObserverEntry],
      {} as IntersectionObserver,
    ),
  );
}

it('loads lazily once visible, converts the managed path and releases the observer', async () => {
  invoke.mockResolvedValue('managed/preview.png');
  const view = render(<VideoPreview asset={asset('lazy')} />);
  expect(invoke).not.toHaveBeenCalled();
  expect(observers[0].observe).toHaveBeenCalledOnce();
  intersect(0, false);
  expect(invoke).not.toHaveBeenCalled();
  intersect();
  const image = await screen.findByRole('img', { name: 'Превью: lazy' });
  expect(image.getAttribute('src')).toBe('asset:managed/preview.png');
  expect(invoke).toHaveBeenCalledWith('get_video_preview_cmd', { artifactId: 'lazy' });
  view.unmount();
  expect(observers[0].disconnect).toHaveBeenCalled();
});

it('uses an avatar for online sources and reports a missing preview', async () => {
  invoke.mockResolvedValue({ dataUrl: null });
  render(<VideoPreview asset={asset('missing-avatar', null)} />);
  intersect();
  expect(await screen.findByText('Превью недоступно')).toBeTruthy();
  expect(invoke).toHaveBeenCalledWith('get_project_avatar_cmd', {
    projectId: 'owner-missing-avatar',
  });
  expect(screen.getByTitle('Миниатюра источника недоступна')).toBeTruthy();
  expect(screen.queryByRole('img')).toBeNull();
});

it('accepts a returned avatar and handles image decode failure without leaving a broken image', async () => {
  invoke.mockResolvedValue({ dataUrl: 'data:image/png;base64,fixture' });
  render(<VideoPreview asset={asset('avatar', null)} />);
  intersect();
  const image = await screen.findByRole('img');
  fireEvent.error(image);
  expect(screen.queryByRole('img')).toBeNull();
  expect(screen.getByTitle('Миниатюра источника недоступна')).toBeTruthy();
});

it('shares one pending preview between cards for the same managed artifact', async () => {
  invoke.mockResolvedValue('managed/shared.png');
  render(
    <>
      <VideoPreview asset={asset('first', 'shared')} />
      <VideoPreview asset={asset('second', 'shared')} compact />
    </>,
  );
  intersect(0);
  intersect(1);
  await waitFor(() => expect(screen.getAllByRole('img')).toHaveLength(2));
  expect(invoke).toHaveBeenCalledOnce();
});

it('evicts failed requests so a later mounted preview can recover', async () => {
  invoke
    .mockRejectedValueOnce({ code: 'NOT_FOUND', message: 'Preview runtime is unavailable' })
    .mockResolvedValueOnce('managed/recovered.png');
  const view = render(<VideoPreview asset={asset('recover-preview')} />);
  intersect();
  await screen.findByTitle('Preview runtime is unavailable');
  expect(screen.queryByRole('img')).toBeNull();
  view.unmount();
  render(<VideoPreview asset={asset('recover-preview')} />);
  intersect(1);
  expect((await screen.findByRole('img')).getAttribute('src')).toBe('asset:managed/recovered.png');
  expect(invoke).toHaveBeenCalledTimes(2);
});

it('serializes native work and ignores a stale response after asset replacement', async () => {
  let resolve!: (path: string) => void;
  invoke
    .mockReturnValueOnce(
      new Promise<string>((yes) => {
        resolve = yes;
      }),
    )
    .mockResolvedValueOnce('managed/current.png');
  const view = render(<VideoPreview asset={asset('stale-preview')} />);
  intersect();
  await waitFor(() => expect(invoke).toHaveBeenCalledOnce());
  view.rerender(<VideoPreview asset={asset('current-preview')} compact />);
  expect(invoke).toHaveBeenCalledOnce();
  await act(async () => {
    resolve('managed/stale.png');
  });
  const image = await screen.findByRole('img', { name: 'Превью: current-preview' });
  expect(image.getAttribute('src')).toBe('asset:managed/current.png');
  expect(invoke).toHaveBeenCalledTimes(2);
});

it('does not render a late preview after unmount', async () => {
  let resolve!: (path: string) => void;
  invoke.mockReturnValue(
    new Promise<string>((yes) => {
      resolve = yes;
    }),
  );
  const view = render(<VideoPreview asset={asset('unmounted-preview')} />);
  intersect();
  await waitFor(() => expect(invoke).toHaveBeenCalledOnce());
  view.unmount();
  await act(async () => {
    resolve('managed/late.png');
  });
  expect(screen.queryByRole('img')).toBeNull();
});
