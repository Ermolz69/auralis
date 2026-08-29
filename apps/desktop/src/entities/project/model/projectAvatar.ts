export const projectAvatarMaxFileSize = 5 * 1024 * 1024;
export const projectAvatarMaxStoredSize = 256 * 1024;
export const projectAvatarSize = 256;

const supportedTypes = ['image/png', 'image/jpeg', 'image/webp', 'image/gif'] as const;
type SupportedImageType = (typeof supportedTypes)[number];

export class ProjectAvatarError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'ProjectAvatarError';
  }
}

export async function normalizeProjectAvatar(file: File): Promise<string> {
  await validateProjectAvatar(file);

  const objectUrl = URL.createObjectURL(file);
  try {
    const image = await loadImage(objectUrl);
    const width = image.naturalWidth;
    const height = image.naturalHeight;
    if (width <= 0 || height <= 0) {
      throw new ProjectAvatarError('Изображение имеет некорректные размеры');
    }

    const canvas = document.createElement('canvas');
    canvas.width = projectAvatarSize;
    canvas.height = projectAvatarSize;
    const context = canvas.getContext('2d');
    if (!context) throw new ProjectAvatarError('Не удалось обработать изображение');

    const sourceSize = Math.min(width, height);
    const sourceX = (width - sourceSize) / 2;
    const sourceY = (height - sourceSize) / 2;
    context.drawImage(
      image,
      sourceX,
      sourceY,
      sourceSize,
      sourceSize,
      0,
      0,
      projectAvatarSize,
      projectAvatarSize,
    );

    const normalized = canvas.toDataURL('image/webp', 0.82);
    if (!normalized.startsWith('data:image/webp;base64,')) {
      throw new ProjectAvatarError('Формат WebP не поддерживается системой');
    }
    if (dataUrlByteLength(normalized) > projectAvatarMaxStoredSize) {
      throw new ProjectAvatarError('Не удалось уменьшить аватарку до допустимого размера');
    }
    return normalized;
  } finally {
    URL.revokeObjectURL(objectUrl);
  }
}

export async function validateProjectAvatar(file: File): Promise<void> {
  if (file.size > projectAvatarMaxFileSize) {
    throw new ProjectAvatarError('Аватарка должна быть не больше 5 МБ');
  }
  if (!isSupportedType(file.type)) {
    throw new ProjectAvatarError('Поддерживаются только PNG, JPEG, WebP и GIF');
  }

  const header = new Uint8Array(await readAsArrayBuffer(file.slice(0, 12)));
  const detectedType = detectImageType(header);
  if (!detectedType || detectedType !== file.type) {
    throw new ProjectAvatarError('Тип файла не соответствует содержимому изображения');
  }
}

function isSupportedType(type: string): type is SupportedImageType {
  return supportedTypes.some((supportedType) => supportedType === type);
}

function detectImageType(bytes: Uint8Array): SupportedImageType | null {
  if (
    bytes.length >= 8 &&
    bytes[0] === 0x89 &&
    bytes[1] === 0x50 &&
    bytes[2] === 0x4e &&
    bytes[3] === 0x47 &&
    bytes[4] === 0x0d &&
    bytes[5] === 0x0a &&
    bytes[6] === 0x1a &&
    bytes[7] === 0x0a
  ) {
    return 'image/png';
  }
  if (bytes.length >= 3 && bytes[0] === 0xff && bytes[1] === 0xd8 && bytes[2] === 0xff) {
    return 'image/jpeg';
  }
  if (bytes.length >= 12 && ascii(bytes, 0, 4) === 'RIFF' && ascii(bytes, 8, 12) === 'WEBP') {
    return 'image/webp';
  }
  if (bytes.length >= 6 && ['GIF87a', 'GIF89a'].includes(ascii(bytes, 0, 6))) {
    return 'image/gif';
  }
  return null;
}

function ascii(bytes: Uint8Array, start: number, end: number): string {
  return String.fromCharCode(...bytes.slice(start, end));
}

function readAsArrayBuffer(blob: Blob): Promise<ArrayBuffer> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => {
      if (reader.result instanceof ArrayBuffer) resolve(reader.result);
      else reject(new ProjectAvatarError('Не удалось прочитать изображение'));
    };
    reader.onerror = () => reject(new ProjectAvatarError('Не удалось прочитать изображение'));
    reader.readAsArrayBuffer(blob);
  });
}

function loadImage(source: string): Promise<HTMLImageElement> {
  return new Promise((resolve, reject) => {
    const image = new Image();
    image.onload = () => resolve(image);
    image.onerror = () =>
      reject(new ProjectAvatarError('Файл не является корректным изображением'));
    image.src = source;
  });
}

function dataUrlByteLength(dataUrl: string): number {
  const base64 = dataUrl.slice(dataUrl.indexOf(',') + 1);
  const padding = base64.endsWith('==') ? 2 : base64.endsWith('=') ? 1 : 0;
  return Math.floor((base64.length * 3) / 4) - padding;
}
