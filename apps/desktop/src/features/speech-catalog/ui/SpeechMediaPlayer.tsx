import { useRef, useState } from 'react';

export function SpeechMediaPlayer({
  src,
  label,
  video = false,
}: {
  src: string;
  label: string;
  video?: boolean;
}) {
  const media = useRef<HTMLMediaElement | null>(null);
  const [error, setError] = useState<string | null>(null);
  const play = async () => {
    setError(null);
    try {
      await media.current?.play();
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    }
  };
  const props = {
    controls: true,
    autoPlay: true,
    src,
    'aria-label': label,
    ref: (node: HTMLMediaElement | null) => {
      media.current = node;
    },
  };
  return (
    <div className="space-y-2">
      <button
        className="rounded border border-border px-3 py-2 text-xs text-text"
        onClick={() => void play()}
      >
        Воспроизвести: {label}
      </button>
      {video ? <video {...props} className="max-h-96 w-full" /> : <audio {...props} />}
      {error && <p role="alert">{error}</p>}
    </div>
  );
}
