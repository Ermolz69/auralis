// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from 'vitest';
import {
  normalizeProjectAvatar,
  projectAvatarMaxFileSize,
  projectAvatarSize,
  validateProjectAvatar,
} from './projectAvatar';

const headers = {
  'image/png': [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a],
  'image/jpeg': [0xff, 0xd8, 0xff, 0xe0],
  'image/webp': [0x52, 0x49, 0x46, 0x46, 0, 0, 0, 0, 0x57, 0x45, 0x42, 0x50],
  'image/gif': [0x47, 0x49, 0x46, 0x38, 0x39, 0x61],
} as const;

afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

describe('project avatar validation', () => {
  it.each(Object.entries(headers))('accepts a valid %s signature', async (type, header) => {
    const file = new File([new Uint8Array(header)], 'avatar', { type });

    await expect(validateProjectAvatar(file)).resolves.toBeUndefined();
  });

  it('rejects files larger than 5 MB', async () => {
    const file = new File([new Uint8Array(projectAvatarMaxFileSize + 1)], 'avatar.png', {
      type: 'image/png',
    });

    await expect(validateProjectAvatar(file)).rejects.toThrow('не больше 5 МБ');
  });

  it('rejects unsupported declared types', async () => {
    const file = new File(['<svg/>'], 'avatar.svg', { type: 'image/svg+xml' });

    await expect(validateProjectAvatar(file)).rejects.toThrow(
      'Поддерживаются только PNG, JPEG, WebP и GIF',
    );
  });

  it('rejects a declared type that does not match the file signature', async () => {
    const file = new File([new Uint8Array(headers['image/jpeg'])], 'spoofed.png', {
      type: 'image/png',
    });

    await expect(validateProjectAvatar(file)).rejects.toThrow(
      'Тип файла не соответствует содержимому',
    );
  });
});

describe('project avatar normalization', () => {
  it('center-crops the image into a 256px WebP and revokes its object URL', async () => {
    const drawImage = vi.fn();
    const canvas = {
      width: 0,
      height: 0,
      getContext: vi.fn(() => ({ drawImage })),
      toDataURL: vi.fn(() => 'data:image/webp;base64,YXZhdGFy'),
    };
    const originalCreateElement = document.createElement.bind(document);
    vi.spyOn(document, 'createElement').mockImplementation((tagName, options) =>
      tagName === 'canvas'
        ? (canvas as unknown as HTMLCanvasElement)
        : originalCreateElement(tagName, options),
    );
    const createObjectURL = vi.fn(() => 'blob:avatar');
    const revokeObjectURL = vi.fn();
    Object.defineProperty(URL, 'createObjectURL', { configurable: true, value: createObjectURL });
    Object.defineProperty(URL, 'revokeObjectURL', { configurable: true, value: revokeObjectURL });
    vi.stubGlobal(
      'Image',
      class {
        naturalWidth = 640;
        naturalHeight = 480;
        onload: (() => void) | null = null;

        set src(_value: string) {
          queueMicrotask(() => this.onload?.());
        }
      },
    );
    const file = new File([new Uint8Array(headers['image/png'])], 'avatar.png', {
      type: 'image/png',
    });

    await expect(normalizeProjectAvatar(file)).resolves.toBe('data:image/webp;base64,YXZhdGFy');
    expect(canvas.width).toBe(projectAvatarSize);
    expect(canvas.height).toBe(projectAvatarSize);
    expect(drawImage).toHaveBeenCalledWith(
      expect.anything(),
      80,
      0,
      480,
      480,
      0,
      0,
      projectAvatarSize,
      projectAvatarSize,
    );
    expect(canvas.toDataURL).toHaveBeenCalledWith('image/webp', 0.82);
    expect(createObjectURL).toHaveBeenCalledWith(file);
    expect(revokeObjectURL).toHaveBeenCalledWith('blob:avatar');
  });
});
