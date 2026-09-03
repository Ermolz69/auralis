const eventName = 'auralis:youtube-imports-changed';

export function youtubeImportsChanged() {
  window.dispatchEvent(new Event(eventName));
}

export function subscribeYoutubeImports(listener: () => void) {
  window.addEventListener(eventName, listener);
  return () => window.removeEventListener(eventName, listener);
}
