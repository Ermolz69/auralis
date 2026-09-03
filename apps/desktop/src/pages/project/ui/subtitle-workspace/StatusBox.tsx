export function StatusBox({ text, danger = false }: { text: string; danger?: boolean }) {
  return (
    <div
      className={`rounded-md border px-3 py-4 text-xs ${
        danger
          ? 'border-danger/50 bg-danger/10 text-danger'
          : 'border-border bg-surface text-subtle'
      }`}
      role={danger ? 'alert' : 'status'}
    >
      {text}
    </div>
  );
}
