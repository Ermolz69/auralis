export function formatSubtitleRange(startMs: number, endMs: number): string {
  return `${formatTimestamp(startMs)} → ${formatTimestamp(endMs)}`;
}

function formatTimestamp(value: number): string {
  const minutes = Math.floor(value / 60_000);
  const seconds = Math.floor((value % 60_000) / 1000);
  const milliseconds = value % 1000;
  return `${String(minutes).padStart(2, '0')}:${String(seconds).padStart(2, '0')}.${String(milliseconds).padStart(3, '0')}`;
}
