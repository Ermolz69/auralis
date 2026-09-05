import { createWindowEventChannel } from '@/shared/lib';

const eventName = 'auralis:youtube-imports-changed';
const youtubeImportChanges = createWindowEventChannel<void>(eventName);

export function youtubeImportsChanged() {
  youtubeImportChanges.emit();
}

export function subscribeYoutubeImports(listener: () => void) {
  return youtubeImportChanges.subscribe(listener);
}
